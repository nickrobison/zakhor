//! Project and Repository Association (Phase 2.4)
//!
//! Associates entities and decisions with `zakhor:Project` /
//! `zakhor:Repository` via `zakhor:belongsToProject` /
//! `zakhor:belongsToRepository`. A project or repository is a tagged
//! collection of related knowledge — an agent can create one and then link
//! any entity or decision to it.

use gio::Cancellable;
use oxrdf::Literal;
use tracker::SparqlConnection;
use tracker::prelude::{SparqlConnectionExtManual, SparqlCursorExtManual};
use zakhor_common::vocab;
use zakhor_storage::sparql::Prefix;
use zakhor_storage::sparql::{format_iri, prefix_declarations};

/// A named project in the knowledge graph.
#[derive(Clone, Debug)]
pub struct Project {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
}

/// A named code repository in the knowledge graph.
#[derive(Clone, Debug)]
pub struct Repository {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
}

/// Build an `INSERT DATA` query creating a typed node with an
/// English-tagged label and optional English comment.
pub(crate) fn build_create_node_sparql(
    class_iri: &str,
    node_uri: &str,
    label: &str,
    description: Option<&str>,
) -> Result<String, String> {
    let uri = format_iri(node_uri).map_err(|e| e.to_string())?;
    let class = format_iri(class_iri).map_err(|e| e.to_string())?;
    let name = Literal::new_language_tagged_literal(label.to_string(), "en")
        .map_err(|e| format!("Invalid name: {e}"))?;
    let desc_clause = match description {
        Some(desc) => format!(
            "  {uri} rdfs:comment {} .\n",
            Literal::new_language_tagged_literal(desc.to_string(), "en")
                .map_err(|e| format!("Invalid description: {e}"))?
        ),
        None => String::new(),
    };
    Ok(format!(
        r#"{prefixes}INSERT DATA {{
  {uri} rdf:type {class} .
  {uri} rdfs:label {name} .
{desc_clause}}}"#,
        prefixes = prefix_declarations(),
    ))
}

/// Build an `INSERT DATA` query linking two resources via a predicate IRI.
pub(crate) fn build_link_sparql(
    predicate_iri: &str,
    from_uri: &str,
    to_uri: &str,
) -> Result<String, String> {
    Ok(format!(
        r#"{prefixes}INSERT DATA {{
  {from} {predicate} {to} .
}}"#,
        prefixes = prefix_declarations(),
        predicate = format_iri(predicate_iri).map_err(|e| e.to_string())?,
        from = checked_iri(from_uri, "link source")?,
        to = checked_iri(to_uri, "link target")?,
    ))
}

/// Format a caller-supplied IRI for interpolation, or explain why it cannot be.
///
/// The previous behaviour stripped `<` and `>`, which kept the query framing
/// intact but still forwarded whitespace, quotes and NUL bytes into the
/// statement. Those reached Tracker as a parse error naming a byte offset, or —
/// for a NUL — panicked inside the GString conversion.
fn checked_iri(uri: &str, what: &str) -> Result<String, String> {
    format_iri(uri).map_err(|_| format!("{what} {uri:?} is not a valid URI"))
}

struct ExistingNode {
    uri: String,
    description: Option<String>,
}

/// Find a node of `class_iri` whose `rdfs:label` is exactly `name`.
///
/// Matching on the label rather than the URI is deliberate: two different names
/// can slugify to the same URI, so a URI probe would report a false match and
/// hand back a node the caller never asked for.
fn find_named_node(
    conn: &SparqlConnection,
    class_iri: &str,
    name: &str,
) -> Result<Option<ExistingNode>, String> {
    let label = Literal::new_language_tagged_literal(name.to_string(), "en")
        .map_err(|e| format!("Invalid name: {e}"))?;

    let sparql = format!(
        r#"{}SELECT ?uri ?comment WHERE {{
  ?uri rdf:type {class} ;
       rdfs:label {label} .
  OPTIONAL {{ ?uri rdfs:comment ?comment . }}
}}"#,
        prefix_declarations(),
        class = format_iri(class_iri).map_err(|e| e.to_string())?,
    );

    let cursor = conn
        .query(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Lookup by name failed: {e}"))?;

    // Drain the cursor before returning: leaving a Tracker statement open
    // blocks the next operation on the same connection.
    let mut found = None;
    while cursor
        .next(None::<&Cancellable>)
        .map_err(|e| format!("Cursor error: {e}"))?
    {
        if found.is_none()
            && let Some(uri) = cursor.string(0)
        {
            found = Some(ExistingNode {
                uri: uri.to_string(),
                description: cursor
                    .string(1)
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty()),
            });
        }
    }

    Ok(found)
}

/// Whether `uri` already names a node in the graph.
///
/// The predicate is deliberately bound. A pattern with a bound subject and an
/// unbound predicate *and* object (`{ <uri> ?p ?o }`) is rejected by Tracker with
/// a bare "SQL logic error" while a store's ontology is still being applied —
/// which is exactly the state a freshly created database is in. Every node this
/// guards is created through a typed insert, so `rdf:type` is a sound witness of
/// existence.
///
/// `ASK` is not usable here: it reports true even for a subject with no triples
/// at all, which would make every candidate URI look taken.
fn uri_exists(conn: &SparqlConnection, uri: &str) -> bool {
    let Ok(subject) = format_iri(uri) else {
        return false;
    };
    let sparql = format!(
        "{}SELECT ?o WHERE {{ {subject} rdf:type ?o }} LIMIT 1",
        prefix_declarations(),
    );

    let probe = || -> Result<bool, String> {
        let cursor = conn
            .query(&sparql, None::<&Cancellable>)
            .map_err(|e| e.to_string())?;
        let mut any = false;
        while cursor
            .next(None::<&Cancellable>)
            .map_err(|e| e.to_string())?
        {
            any = true;
        }
        Ok(any)
    };

    match probe() {
        Ok(found) => found,
        Err(e) => {
            // This check only avoids a needless collision; the insert that
            // follows is the authority and reports collisions precisely. A
            // probe that cannot answer must not fail the operation.
            tracing::warn!("URI existence probe failed for {uri} ({e}); assuming the URI is free");
            false
        }
    }
}

/// Pick an unused URI under `prefix` for a node called `name`.
///
/// The plain slug is preferred so URIs stay human-readable and derivable. Only
/// when distinct names slugify identically is a numeric suffix added, which
/// keeps both nodes addressable instead of letting the second overwrite the
/// first.
fn reserve_node_uri(conn: &SparqlConnection, prefix: &str, name: &str) -> Result<String, String> {
    let base = format!("{prefix}{}", slugify(name));
    if !uri_exists(conn, &base) {
        return Ok(base);
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !uri_exists(conn, &candidate) {
            return Ok(candidate);
        }
        suffix += 1;
    }
}

fn warn_if_description_ignored(kind: &str, name: &str, description: Option<&str>) {
    if description.is_some() {
        tracing::warn!(
            "create_{kind} called for the existing {kind} {name:?} with a description; \
             descriptions are only applied at creation, so the stored one is kept"
        );
    }
}

/// Reject a link whose subject is not in the graph.
///
/// Without this the failure surfaces as Tracker's own ontology diagnostic, whose
/// wording is garbled ("is not is not a rdfs:Resource") and which leaks
/// storage-layer phrasing to the caller. The useful fact is simply that the
/// subject does not exist yet.
fn require_existing_subject(
    conn: &SparqlConnection,
    subject_uri: &str,
    target_description: &str,
) -> Result<(), String> {
    if uri_exists(conn, subject_uri) {
        return Ok(());
    }
    Err(format!(
        "Cannot link {subject_uri} to {target_description}: no such node in the graph. \
         Store it first, or check the URI."
    ))
}

/// Create a new project and insert it into the graph.
///
/// Idempotent by name: if a project carrying this `name` already exists it is
/// returned unchanged rather than re-inserted. The URI is derived from the
/// name, so re-inserting would target the same subject and Tracker would reject
/// the second value written to the single-valued `rdfs:comment`. Agents re-issue
/// `create_*` calls whenever they are unsure a node exists, so treating the
/// duplicate as an error strands the caller; returning the existing node keeps
/// the URI stable and predictable. A supplied `description` therefore only
/// applies at creation — it is not written to an existing project.
pub fn create_project(
    conn: &SparqlConnection,
    name: &str,
    description: Option<&str>,
) -> Result<Project, String> {
    if let Some(existing) = find_named_node(conn, vocab::project_iri().as_str(), name)? {
        warn_if_description_ignored("project", name, description);
        return Ok(Project {
            uri: existing.uri,
            name: name.to_string(),
            description: existing.description,
        });
    }

    let project_uri = reserve_node_uri(conn, &format!("{}project/", Prefix::ZAKHOR), name)?;
    let sparql = build_create_node_sparql(
        vocab::project_iri().as_str(),
        &project_uri,
        name,
        description,
    )?;

    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Create project failed: {e}"))?;

    Ok(Project {
        uri: project_uri,
        name: name.to_string(),
        description: description.map(String::from),
    })
}

/// Link an entity or decision to a project via `zakhor:belongsToProject`.
pub fn link_to_project(
    conn: &SparqlConnection,
    entity_uri: &str,
    project_uri: &str,
) -> Result<(), String> {
    require_existing_subject(conn, entity_uri, "a project")?;
    let sparql = build_link_sparql(
        vocab::belongs_to_project_iri().as_str(),
        entity_uri,
        project_uri,
    )?;
    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Link to project failed: {e}"))?;
    Ok(())
}

/// Create a new repository and insert it into the graph.
///
/// Idempotent by name, for the same reason and with the same trade-off as
/// [`create_project`].
pub fn create_repository(
    conn: &SparqlConnection,
    name: &str,
    description: Option<&str>,
) -> Result<Repository, String> {
    if let Some(existing) = find_named_node(conn, vocab::repository_iri().as_str(), name)? {
        warn_if_description_ignored("repository", name, description);
        return Ok(Repository {
            uri: existing.uri,
            name: name.to_string(),
            description: existing.description,
        });
    }

    let repository_uri = reserve_node_uri(conn, &format!("{}repository/", Prefix::ZAKHOR), name)?;
    let sparql = build_create_node_sparql(
        vocab::repository_iri().as_str(),
        &repository_uri,
        name,
        description,
    )?;

    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Create repository failed: {e}"))?;

    Ok(Repository {
        uri: repository_uri,
        name: name.to_string(),
        description: description.map(String::from),
    })
}

/// Link an entity to a repository via `zakhor:belongsToRepository`.
pub fn link_to_repository(
    conn: &SparqlConnection,
    entity_uri: &str,
    repository_uri: &str,
) -> Result<(), String> {
    require_existing_subject(conn, entity_uri, "a repository")?;
    let sparql = build_link_sparql(
        vocab::belongs_to_repository_iri().as_str(),
        entity_uri,
        repository_uri,
    )?;
    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Link to repository failed: {e}"))?;
    Ok(())
}

/// List all projects.
pub fn list_projects(conn: &SparqlConnection) -> Result<Vec<Project>, String> {
    let sparql = format!(
        r#"{}SELECT ?uri ?label ?comment WHERE {{
  ?uri rdf:type zakhor:Project .
  ?uri rdfs:label ?label .
  OPTIONAL {{ ?uri rdfs:comment ?comment . }}
}}
ORDER BY ?label"#,
        prefix_declarations(),
    );

    let cursor = conn
        .query(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("List projects failed: {e}"))?;

    let mut projects = Vec::new();
    while cursor
        .next(None::<&Cancellable>)
        .map_err(|e| format!("Cursor error: {e}"))?
    {
        let uri = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let name = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        let desc = cursor.string(2).map(|s| s.to_string());
        projects.push(Project {
            uri,
            name,
            description: desc.filter(|s| !s.is_empty()),
        });
    }

    Ok(projects)
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .filter(|c| *c != '\'')
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tracker needs a real store on disk, so these exercise the actual insert
    /// path rather than a mock.
    fn test_store(tag: &str) -> SparqlConnection {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "zakhor-project-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&path);
        zakhor_storage::tracker_db::init_db(path.to_str().expect("utf-8 temp path"))
    }

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("My Project"), "my-project");
        assert_eq!(slugify("Hello World!"), "hello-world");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify("  Spaces  "), "spaces");
        assert_eq!(slugify("a/b\\c"), "a-b-c");
    }

    #[test]
    fn test_project_struct() {
        let p = Project {
            uri: "http://zakhor/ns/project/test".into(),
            name: "Test".into(),
            description: Some("A test project".into()),
        };
        assert_eq!(p.name, "Test");
        assert_eq!(p.description.as_deref(), Some("A test project"));
    }

    #[test]
    fn test_repository_struct() {
        let r = Repository {
            uri: "http://zakhor/ns/repository/test".into(),
            name: "Test".into(),
            description: None,
        };
        assert_eq!(r.name, "Test");
        assert_eq!(r.description, None);
    }

    #[test]
    fn test_build_create_node_sparql_project() {
        let query = build_create_node_sparql(
            "http://zakhor/ns/Project",
            "http://zakhor/ns/project/test",
            "Test Project",
            Some("A test project"),
        )
        .expect("valid IRIs");
        assert!(query.contains("rdf:type <http://zakhor/ns/Project>"));
        assert!(query.contains("\"Test Project\"@en"));
        assert!(query.contains("rdfs:comment"));
        assert!(query.contains("\"A test project\"@en"));

        let no_desc = build_create_node_sparql(
            "http://zakhor/ns/Project",
            "http://zakhor/ns/project/test",
            "Test Project",
            None,
        )
        .expect("valid IRIs");
        assert!(no_desc.contains("rdf:type <http://zakhor/ns/Project>"));
        assert!(!no_desc.contains("rdfs:comment"));
    }

    /// The existence probe must bind its predicate.
    ///
    /// `{ <uri> ?p ?o }` — bound subject, unbound predicate *and* object — is
    /// rejected by Tracker with a bare "SQL logic error" while a store's
    /// ontology is still loading, which broke every `create_project` call on a
    /// freshly created database in CI while passing locally. The probe is
    /// pinned to `rdf:type`, which every node this guards carries.
    #[test]
    fn test_uri_probe_binds_its_predicate() {
        let conn = test_store("uri-probe-shape");
        let free = "http://zakhor/ns/project/never-created";
        let taken = "http://zakhor/ns/project/taken";

        conn.update(
            format!("INSERT DATA {{ <{taken}> rdf:type <http://zakhor/ns/Project> . }}").as_str(),
            None::<&Cancellable>,
        )
        .expect("insert");

        assert!(!uri_exists(&conn, free), "an unused URI must read as free");
        assert!(uri_exists(&conn, taken), "a typed node must read as taken");
    }

    /// A probe that cannot answer must not take the caller down with it.
    ///
    /// The check is an optimisation that avoids a needless collision; the
    /// insert that follows is the authority. This asserts the signature returns
    /// a plain `bool`, so there is no error path left to propagate.
    #[test]
    fn test_uri_probe_cannot_fail_the_caller() {
        let _: fn(&SparqlConnection, &str) -> bool = uri_exists;
    }

    /// Creating the same project twice must not fail, and must hand back the
    /// same URI. The descriptions differ deliberately: Tracker's
    /// single-valued-property rejection only fires when the second value
    /// actually differs, so a test using equal descriptions would pass against
    /// the bug.
    #[test]
    fn test_create_project_is_idempotent_by_name() {
        let conn = test_store("create-project-idempotent");

        let first = create_project(&conn, "dup", Some("first")).expect("first create");
        let second = create_project(&conn, "dup", Some("second")).expect("duplicate create");

        assert_eq!(
            first.uri, second.uri,
            "a repeated create must return the existing project, not a new URI"
        );
        assert_eq!(first.uri, format!("{}project/dup", Prefix::ZAKHOR));
    }

    #[test]
    fn test_create_repository_is_idempotent_by_name() {
        let conn = test_store("create-repo-idempotent");

        let first = create_repository(&conn, "dup", Some("first")).expect("first create");
        let second = create_repository(&conn, "dup", Some("second")).expect("duplicate create");

        assert_eq!(first.uri, second.uri);
    }

    /// Distinct names that slugify identically must both survive: they are
    /// different projects and must get different addresses.
    #[test]
    fn test_names_sharing_a_slug_get_distinct_uris() {
        let conn = test_store("slug-collision");

        let spaced = create_project(&conn, "My Project", Some("a")).expect("first create");
        let slashed = create_project(&conn, "my/project", Some("b")).expect("second create");

        assert_eq!(spaced.uri, format!("{}project/my-project", Prefix::ZAKHOR));
        assert_ne!(
            spaced.uri, slashed.uri,
            "distinct names must not collapse onto one node"
        );
        assert_eq!(
            slashed.uri,
            format!("{}project/my-project-2", Prefix::ZAKHOR)
        );
    }

    #[test]
    fn test_slug_collision_disambiguates_repositories_too() {
        let conn = test_store("slug-collision-repo");

        let a = create_repository(&conn, "My Repo", None).expect("first create");
        let b = create_repository(&conn, "my/repo", None).expect("second create");

        assert_ne!(a.uri, b.uri);
    }

    /// A link to a subject that was never stored must fail with an actionable
    /// message, not Tracker's ontology diagnostic.
    #[test]
    fn test_link_to_missing_subject_reports_a_clean_error() {
        let conn = test_store("link-missing");
        let project = create_project(&conn, "p", None).expect("create project");

        let err = link_to_project(&conn, "http://example.com/nope", &project.uri)
            .expect_err("linking an unknown subject must fail");

        assert!(
            err.contains("no such node"),
            "error should explain the subject is absent, got: {err}"
        );
        assert!(
            !err.contains("is not is not"),
            "error must not leak Tracker's garbled diagnostic, got: {err}"
        );
        assert!(
            !err.contains("rdfs:Resource"),
            "error must not leak storage-layer vocabulary, got: {err}"
        );
    }

    #[test]
    fn test_link_to_existing_subject_succeeds() {
        let conn = test_store("link-existing");
        let project = create_project(&conn, "p", None).expect("create project");
        let entity = "http://zakhor/ns/entity/link-target";
        conn.update(
            format!("INSERT DATA {{ <{entity}> rdf:type <http://zakhor/ns/Entity> . }}").as_str(),
            None::<&Cancellable>,
        )
        .expect("insert entity");

        link_to_project(&conn, entity, &project.uri).expect("link must succeed");
    }

    #[test]
    fn test_build_link_sparql_strips_angle_brackets() {
        let query = build_link_sparql(
            "http://zakhor/ns/belongsToProject",
            "http://zakhor/ns/entity/e1",
            "http://zakhor/ns/project/p1",
        )
        .expect("valid IRIs");
        assert_eq!(query.matches("<http://zakhor/ns/entity/e1>").count(), 1);
        assert_eq!(query.matches("<http://zakhor/ns/project/p1>").count(), 1);
        assert!(query.contains("<http://zakhor/ns/belongsToProject>"));
        assert!(!query.contains("<<"));
    }
}
