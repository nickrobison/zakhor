//! Migration binary: Zakhor v1 → v2
//!
//! Migrates existing observations stored as flat triples into v2 named-graph
//! provenance format.  v1 observations use `nie:InformationElement` without
//! an explicit named graph; v2 wraps each observation in `<zakhor:graph/{uuid}>`.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --bin migrate-v1-to-v2 -- --dry-run
//! cargo run --bin migrate-v1-to-v2          # apply
//! cargo run --bin migrate-v1-to-v2 -- --db-path /custom/path
//! ```

use std::path::PathBuf;

use clap::Parser;
use gio::Cancellable;
use tracker::SparqlConnection;
use tracker::prelude::{SparqlConnectionExtManual, SparqlCursorExtManual};

/// SPARQL store configuration.
const GRAPH_PREFIX: &str = "http://zakhor/ns/graph/";

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "migrate-v1-to-v2",
    about = "Migrate Zakhor v1 observations to v2 named graphs"
)]
struct Cli {
    /// Database path for Tracker SPARQL store.
    #[arg(long, default_value = "./zakhor-db")]
    db_path: PathBuf,

    /// Dry-run: print what would be migrated without applying.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Verbose output: show every migrated observation.
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    // Initialise tracking (minimal — no MCP server involved)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let db_path = cli.db_path.to_str().expect("valid db path");
    let conn = init_db(db_path);

    let results = migrate(&conn, cli.dry_run, cli.verbose);
    match results {
        Ok(summary) => {
            tracing::info!(
                scanned = summary.scanned,
                migrated = summary.migrated,
                skipped = summary.skipped,
                errors = summary.errors,
                "Migration complete",
            );
            if summary.errors > 0 {
                std::process::exit(1);
            }
        }
        Err(e) => {
            tracing::error!("Migration failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Summary of migration results.
#[derive(Debug, Default)]
struct MigrationSummary {
    scanned: u64,
    migrated: u64,
    skipped: u64,
    errors: u64,
}

// ---------------------------------------------------------------------------
// Migration logic
// ---------------------------------------------------------------------------

/// Run the v1 → v2 migration.
fn migrate(
    conn: &SparqlConnection,
    dry_run: bool,
    verbose: bool,
) -> Result<MigrationSummary, String> {
    let mut summary = MigrationSummary::default();

    // Step 1: Scan all v1 observations (nie:InformationElement)
    let observations = scan_v1_observations(conn)?;
    summary.scanned = observations.len() as u64;
    tracing::info!(count = summary.scanned, "Scanned v1 observations");

    if observations.is_empty() {
        tracing::info!("No v1 observations found — nothing to migrate");
        return Ok(summary);
    }

    // Step 2: For each observation, copy triples into a named graph
    for (uuid, iri) in &observations {
        if dry_run {
            tracing::info!("[DRY-RUN] Would migrate {} ({})", uuid, iri);
            summary.migrated += 1;
            continue;
        }

        match migrate_observation(conn, uuid, iri, verbose) {
            Ok(true) => {
                summary.migrated += 1;
                if verbose {
                    tracing::info!("Migrated {}", uuid);
                }
            }
            Ok(false) => {
                summary.skipped += 1;
                if verbose {
                    tracing::info!("Skipped {} (already has graph)", uuid);
                }
            }
            Err(e) => {
                tracing::error!("Failed to migrate {}: {}", uuid, e);
                summary.errors += 1;
            }
        }
    }

    Ok(summary)
}

/// Scan the triplestore for all v1 observation IRIs.
///
/// Returns `(uuid_part, full_iri)` pairs extracted from `nie:identifier` values.
fn scan_v1_observations(conn: &SparqlConnection) -> Result<Vec<(String, String)>, String> {
    let sparql = r#"PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>

SELECT DISTINCT ?iri ?id WHERE {
  ?iri rdf:type nie:InformationElement ;
       nie:identifier ?id .
}
ORDER BY ?id"#
        .to_string();

    let cursor = conn
        .query(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Scan query failed: {e}"))?;

    let mut results = Vec::new();
    while cursor
        .next(None::<&Cancellable>)
        .map_err(|e| format!("Cursor error: {e}"))?
    {
        let iri = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let id = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        if iri.is_empty() {
            continue;
        }

        // Extract UUID portion from the identifier (typically urn:uuid:...)
        let uuid = id.strip_prefix("urn:uuid:").unwrap_or(&id).to_string();
        results.push((uuid, iri));
    }

    Ok(results)
}

/// Migrate a single observation into a named graph.
///
/// Returns `Ok(true)` if migrated, `Ok(false)` if the graph already exists.
fn migrate_observation(
    conn: &SparqlConnection,
    uuid: &str,
    iri: &str,
    verbose: bool,
) -> Result<bool, String> {
    let graph_uri = format!("{GRAPH_PREFIX}{uuid}");

    // Check if graph already exists
    let check = format!(r#"ASK {{ GRAPH <{graph_uri}> {{ ?s ?p ?o }} }}"#,);
    let cursor = conn
        .query(&check, None::<&Cancellable>)
        .map_err(|e| format!("Check graph failed: {e}"))?;

    // ASK returns a single boolean result
    let _ = cursor.next(None::<&Cancellable>).unwrap_or(false);
    let already_exists = cursor.is_boolean(0);

    if already_exists {
        return Ok(false);
    }

    // Fetch all triples for this observation
    let triples = fetch_observation_triples(conn, iri)?;
    if triples.is_empty() {
        return Ok(false);
    }

    // Build SPARQL INSERT DATA with named graph
    let mut sparql = String::with_capacity(2048 + triples.len() * 128);
    sparql.push_str("PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n");
    sparql.push_str("INSERT DATA {\n");
    sparql.push_str(&format!("  GRAPH <{}> {{\n", graph_uri));

    for (s, p, o) in &triples {
        if o.starts_with("http://") || o.starts_with("urn:") {
            sparql.push_str(&format!("    <{}> <{}> <{}> .\n", s, p, o));
        } else {
            let escaped = o.replace('\\', "\\\\").replace('"', "\\\"");
            sparql.push_str(&format!(
                "    <{}> <{}> \"{}\"^^xsd:string .\n",
                s, p, escaped
            ));
        }
    }

    sparql.push_str("  }\n");
    sparql.push_str("}\n");

    if verbose {
        tracing::debug!("SPARQL for {}:\n{}", uuid, sparql);
    }

    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("INSERT DATA failed: {e}"))?;

    Ok(true)
}

/// Fetch all triples where `iri` is the subject.
fn fetch_observation_triples(
    conn: &SparqlConnection,
    iri: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let sparql = format!("SELECT ?p ?o WHERE {{ <{}> ?p ?o }}", iri);

    let cursor = conn
        .query(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Fetch triples failed: {e}"))?;

    let mut triples = Vec::new();
    while cursor
        .next(None::<&Cancellable>)
        .map_err(|e| format!("Cursor error: {e}"))?
    {
        let p = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let o = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        if !p.is_empty() {
            triples.push((iri.to_string(), p, o));
        }
    }

    Ok(triples)
}

// ---------------------------------------------------------------------------
// DB initialisation (minimal — same pattern as tracker_db.rs)
// ---------------------------------------------------------------------------

fn init_db(path: &str) -> SparqlConnection {
    use gio::File;
    use tracker::SparqlConnectionFlags;

    let store = File::for_path(path);
    let db_path = std::path::Path::new(path);
    std::fs::create_dir_all(db_path).expect("failed to create db directory");

    let env_var = "TRACKER_ENDPOINT";
    if let Ok(endpoint) = std::env::var(env_var)
        && !endpoint.is_empty()
    {
        tracing::info!("Using Tracker endpoint: {endpoint}");
        return SparqlConnection::new(
            SparqlConnectionFlags::empty(),
            None::<&gio::File>,
            None::<&gio::File>,
            None::<&gio::Cancellable>,
        )
        .expect("failed to connect to Tracker endpoint");
    }

    let result = SparqlConnection::new(
        SparqlConnectionFlags::empty(),
        Some(&store),
        None::<&gio::File>,
        None::<&gio::Cancellable>,
    );
    match result {
        Ok(conn) => {
            tracing::info!("Initialised SPARQL store at {}", path);
            conn
        }
        Err(e) => {
            panic!("Failed to init Tracker DB: {e}");
        }
    }
}
