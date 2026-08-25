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
use zakhor_storage::sparql::prefix_declarations;

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
) -> String {
    let uri = sanitize_uri(node_uri);
    let class = sanitize_uri(class_iri);
    let name = Literal::new_language_tagged_literal(label.to_string(), "en").unwrap();
    let desc_clause = description
        .map(|desc| {
            format!(
                "  <{uri}> rdfs:comment {} .\n",
                Literal::new_language_tagged_literal(desc.to_string(), "en").unwrap()
            )
        })
        .unwrap_or_default();
    format!(
        r#"{prefixes}INSERT DATA {{
  <{uri}> rdf:type <{class}> .
  <{uri}> rdfs:label {name} .
{desc_clause}}}"#,
        prefixes = prefix_declarations(),
    )
}

/// Build an `INSERT DATA` query linking two resources via a predicate IRI.
pub(crate) fn build_link_sparql(predicate_iri: &str, from_uri: &str, to_uri: &str) -> String {
    format!(
        r#"{prefixes}INSERT DATA {{
  <{from}> <{predicate}> <{to}> .
}}"#,
        prefixes = prefix_declarations(),
        predicate = sanitize_uri(predicate_iri),
        from = sanitize_uri(from_uri),
        to = sanitize_uri(to_uri),
    )
}

fn sanitize_uri(uri: &str) -> String {
    uri.chars().filter(|c| *c != '<' && *c != '>').collect()
}

/// Create a new project and insert it into the graph.
pub fn create_project(
    conn: &SparqlConnection,
    name: &str,
    description: Option<&str>,
) -> Result<Project, String> {
    let project_uri = format!("{}project/{}", Prefix::ZAKHOR, slugify(name));
    let sparql = build_create_node_sparql(
        vocab::project_iri().as_str(),
        &project_uri,
        name,
        description,
    );

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
    let sparql = build_link_sparql(
        vocab::belongs_to_project_iri().as_str(),
        entity_uri,
        project_uri,
    );
    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Link to project failed: {e}"))?;
    Ok(())
}

/// Create a new repository and insert it into the graph.
pub fn create_repository(
    conn: &SparqlConnection,
    name: &str,
    description: Option<&str>,
) -> Result<Repository, String> {
    let repository_uri = format!("{}repository/{}", Prefix::ZAKHOR, slugify(name));
    let sparql = build_create_node_sparql(
        vocab::repository_iri().as_str(),
        &repository_uri,
        name,
        description,
    );

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
    let sparql = build_link_sparql(
        vocab::belongs_to_repository_iri().as_str(),
        entity_uri,
        repository_uri,
    );
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
        );
        assert!(query.contains("rdf:type <http://zakhor/ns/Project>"));
        assert!(query.contains("\"Test Project\"@en"));
        assert!(query.contains("rdfs:comment"));
        assert!(query.contains("\"A test project\"@en"));

        let no_desc = build_create_node_sparql(
            "http://zakhor/ns/Project",
            "http://zakhor/ns/project/test",
            "Test Project",
            None,
        );
        assert!(no_desc.contains("rdf:type <http://zakhor/ns/Project>"));
        assert!(!no_desc.contains("rdfs:comment"));
    }

    #[test]
    fn test_build_link_sparql_strips_angle_brackets() {
        let query = build_link_sparql(
            "http://zakhor/ns/belongsToProject",
            "<http://zakhor/ns/entity/e1>",
            "<http://zakhor/ns/project/p1>",
        );
        assert_eq!(query.matches("<http://zakhor/ns/entity/e1>").count(), 1);
        assert_eq!(query.matches("<http://zakhor/ns/project/p1>").count(), 1);
        assert!(query.contains("<http://zakhor/ns/belongsToProject>"));
        assert!(!query.contains("<<"));
    }
}
