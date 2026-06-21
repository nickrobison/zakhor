use gio::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tracker::{SparqlConnection, SparqlConnectionFlags};

/// System ontology resource paths from libtracker-sparql-3.0.so.
///
/// These are compiled into the tracker library as GResources and must be
/// extracted to a real directory because Tracker's ontology loader only
/// accepts `resource://` URIs or directories, not standalone files.
const SYSTEM_ONTOLOGY_RESOURCES: &[&str] = &[
    "/org/freedesktop/tracker/ontology/10-xsd.ontology",
    "/org/freedesktop/tracker/ontology/11-rdf.ontology",
    "/org/freedesktop/tracker/ontology/12-nrl.ontology",
    "/org/freedesktop/tracker/ontology/20-dc.ontology",
    "/org/freedesktop/tracker/nepomuk/30-nie.ontology",
    "/org/freedesktop/tracker/nepomuk/31-nao.ontology",
    "/org/freedesktop/tracker/nepomuk/32-nco.ontology",
    "/org/freedesktop/tracker/nepomuk/33-nfo.ontology",
    "/org/freedesktop/tracker/nepomuk/38-nmm.ontology",
    "/org/freedesktop/tracker/nepomuk/41-mfo.ontology",
    "/org/freedesktop/tracker/nepomuk/90-tracker.ontology",
    "/org/freedesktop/tracker/nepomuk/92-slo.ontology",
    "/org/freedesktop/tracker/nepomuk/93-libosinfo.ontology",
];

/// Extract system ontologies from the tracker library's gresources into
/// `onto_dir`. Returns the number of ontologies successfully written.
fn write_system_ontologies(onto_dir: &Path) -> usize {
    let _ = std::fs::create_dir_all(onto_dir);
    let mut written = 0usize;
    for resource_path in SYSTEM_ONTOLOGY_RESOURCES {
        match gio::resources_lookup_data(resource_path, gio::ResourceLookupFlags::NONE) {
            Ok(bytes) => {
                let file_name = resource_path
                    .rsplit('/')
                    .next()
                    .unwrap_or("unknown.ontology");
                let dest = onto_dir.join(file_name);
                if let Err(e) = std::fs::write(&dest, bytes.as_ref()) {
                    tracing::warn!(error = %e, path = %dest.display(), "Cannot write system ontology");
                } else {
                    written += 1;
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, resource = %resource_path, "Cannot read system ontology from gresource");
            }
        }
    }
    written
}

/// Force the tracker library to register its compiled GResources by creating
/// a throw-away Tracker SPARQL connection.
///
/// Tracker's ontology resources (`/org/freedesktop/tracker/ontology/…` and
/// `/org/freedesktop/tracker/nepomuk/…`) are compiled into
/// `libtracker-sparql-3.0.so` as GLib GResources, but they are not
/// registered until the library's internal initialisation runs — which
/// happens during the first `SparqlConnection::new()` call. Without this
/// warmup, `gio::resources_lookup_data()` returns `NotFound` and ontology
/// extraction yields zero files.
fn warmup_gresources() {
    let pid = std::process::id();
    let warmup_dir = std::env::temp_dir().join(format!("zakhor-warmup-{pid}"));
    let _ = std::fs::create_dir_all(&warmup_dir);

    {
        let _warmup = SparqlConnection::new(
            SparqlConnectionFlags::empty(),
            Some(&File::for_path(&warmup_dir)),
            tracker::functions::sparql_get_ontology_nepomuk().as_ref(),
            None::<&gio::Cancellable>,
        );
    } // connection dropped — gresources remain registered for the process lifetime

    let _ = std::fs::remove_dir_all(&warmup_dir);
}

/// Extract system ontologies from gresources and write the custom zakhor
/// ontology into `db_path/ontologies/`. Returns the ontology source to pass
/// to `SparqlConnection::new()`.
///
/// This may trigger a warmup connection if tracker's gresources aren't
/// registered yet (first call per process lifetime).
fn prepare_ontology_dir(db_path: &Path) -> Option<gio::File> {
    let ontology_dir = db_path.join("ontologies");
    let _ = std::fs::create_dir_all(&ontology_dir);

    let mut n_sys = write_system_ontologies(&ontology_dir);

    if n_sys < SYSTEM_ONTOLOGY_RESOURCES.len() {
        tracing::info!("System ontology gresources not yet registered — performing warmup");
        warmup_gresources();
        n_sys = write_system_ontologies(&ontology_dir);
    }

    if n_sys < SYSTEM_ONTOLOGY_RESOURCES.len() {
        tracing::warn!(
            n_written = n_sys,
            n_expected = SYSTEM_ONTOLOGY_RESOURCES.len(),
            "Only {}/{} system ontologies extracted — some standard classes may be unavailable",
            n_sys,
            SYSTEM_ONTOLOGY_RESOURCES.len(),
        );
    }

    // Always write the custom zakhor ontology
    let custom_path = ontology_dir.join("99-zakhor.ontology");
    if let Err(e) = std::fs::write(&custom_path, crate::schema::ontology_file_content()) {
        tracing::warn!(error = %e, "Failed to write custom ontology file");
    }

    if n_sys > 0 {
        tracing::info!(
            dir = %ontology_dir.display(),
            "Using combined ontology directory ({} system + zakhor)",
            n_sys,
        );
        Some(File::for_path(&ontology_dir))
    } else {
        tracing::warn!(
            "No system ontologies extracted — falling back to resource-based Nepomuk \
             ontology. Zakhor-specific types will not have SQL tables."
        );
        tracker::functions::sparql_get_ontology_nepomuk()
    }
}

/// Track whether we've already performed an ontology-drift recovery in this
/// process lifetime. Single retry is enough — if it fails again the error is
/// real (e.g. corrupt disk) and should propagate.
static DID_ONTOLOGY_RETRY: AtomicBool = AtomicBool::new(false);

/// Open (or create) a Tracker SPARQL database at `path`, loading both the
/// standard Nepomuk ontologies and Zakhor's custom ontology classes and
/// properties into the SQL schema.
///
/// The returned `SparqlConnection` provides full SPARQL read/write access
/// to the RDF store, with Zakhor-specific types (`zakhor:Entity`,
/// `zakhor:Decision`, …) registered as queryable classes.
///
/// # Ontology schema drift recovery
///
/// Tracker does not support changing `rdfs:domain` or `rdfs:range` on an
/// already-registered property. If the current code's ontology file disagrees
/// with the persisted database, `init_db` automatically clears the stale
/// store and retries once. If the retry also fails, the error is propagated.
///
/// # Gresource warmup
///
/// On the first call in a process, tracker's compiled-in ontology gresources
/// may not be registered yet. This function transparently creates a short-lived
/// temporary Tracker connection to force library initialisation, then retries
/// the extraction, so callers always get a fully-populated combined ontology
/// directory in a single invocation.
pub fn init_db(path: &str) -> SparqlConnection {
    let store = File::for_path(path);
    let db_path = Path::new(path);
    let _ = std::fs::create_dir_all(db_path);

    let ontology_file = prepare_ontology_dir(db_path);

    let result = SparqlConnection::new(
        SparqlConnectionFlags::empty(),
        Some(&store),
        ontology_file.as_ref(),
        None::<&gio::Cancellable>,
    );

    match result {
        Ok(conn) => conn,
        Err(e) => {
            let msg = format!("{e}");
            // Tracker rejects ontology evolution (rdfs:domain / rdfs:range changes)
            if msg.contains("is not supported") && !DID_ONTOLOGY_RETRY.swap(true, Ordering::SeqCst)
            {
                tracing::warn!(
                    error = %e,
                    "Ontology schema drift detected — clearing stale database and retrying"
                );
                // Nuke the entire store so Tracker can re-create it with the current ontology
                let _ = std::fs::remove_dir_all(db_path);
                let _ = std::fs::create_dir_all(db_path);
                // Re-prepare ontology files in the clean directory
                let ontology_file = prepare_ontology_dir(db_path);
                SparqlConnection::new(
                    SparqlConnectionFlags::empty(),
                    Some(&store),
                    ontology_file.as_ref(),
                    None::<&gio::Cancellable>,
                )
                .expect("Failed to init Tracker DB after clearing stale store")
            } else {
                panic!("Failed to init Tracker DB: {e}");
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tracker::prelude::{SparqlConnectionExtManual, SparqlCursorExtManual};

    /// Create a Tracker connection then INSERT/query a zakhor:Entity to verify
    /// the ontology pipeline actually produces a working store.
    #[test]
    fn test_entity_type_query_and_insert() {
        let tmp = std::env::temp_dir().join(format!("zakhor-inttest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        let conn = init_db(tmp.to_str().unwrap());

        // Phase 1 — type query on empty DB must not crash
        let cursor = conn
            .query(
                "SELECT ?s WHERE { ?s a <http://zakhor/ns/Entity> } LIMIT 1",
                None::<&gio::Cancellable>,
            )
            .expect("SHOULD-1: SELECT by zakhor:Entity type must not fail");

        let hit_empty = cursor.next(None::<&gio::Cancellable>).unwrap_or(false);
        assert!(!hit_empty, "Fresh DB must have zero zakhor:Entity rows");

        // Phase 2 — INSERT a zakhor:Entity
        let uuid =
            tracker::functions::sparql_get_uuid_urn().expect("SHOULD-2: UUID generation must work");
        let insert = format!(
            "INSERT DATA {{ \
             <{uuid}> a <http://zakhor/ns/Entity> ; \
                       rdfs:label \"integration probe\"@en . \
             }}"
        );
        conn.update(&insert, None::<&gio::Cancellable>)
            .expect("SHOULD-3: INSERT of zakhor:Entity must succeed");

        // Phase 3 — read label back
        let cursor2 = conn
            .query(
                &format!("SELECT ?l WHERE {{ <{uuid}> rdfs:label ?l }}"),
                None::<&gio::Cancellable>,
            )
            .expect("SHOULD-4: Label query must work");
        assert!(
            cursor2.next(None::<&gio::Cancellable>).unwrap(),
            "SHOULD-5: Must find one label row"
        );
        let label = cursor2.string(0).expect("SHOULD-6: Label must be non-null");
        assert_eq!(label, "integration probe", "SHOULD-7: Label must match");

        // Phase 4 — re-query by type
        let cursor3 = conn
            .query(
                "SELECT ?s WHERE { ?s a <http://zakhor/ns/Entity> }",
                None::<&gio::Cancellable>,
            )
            .expect("SHOULD-8: Second type query must work");
        assert!(
            cursor3.next(None::<&gio::Cancellable>).unwrap(),
            "SHOULD-9: Must find the inserted entity by type"
        );
        let found_uri = cursor3.string(0).expect("SHOULD-10: URI must be non-null");
        assert_eq!(
            found_uri, uuid,
            "SHOULD-11: Found URI must match inserted UUID"
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
