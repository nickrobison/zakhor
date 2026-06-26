//! Project Association (Phase 2.4)
//!
//! Associates entities and decisions with `memory:Project` via
//! `memory:belongsToProject`.  A project is a tagged collection of related
//! knowledge — an agent can create a project and then link any entity or
//! decision to it.

use gio::Cancellable;
use tracker::prelude::{SparqlConnectionExtManual, SparqlCursorExtManual};
use tracker::SparqlConnection;
use zakhor_storage::sparql::Prefix;

/// A named project in the knowledge graph.
#[derive(Clone, Debug)]
pub struct Project {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
}

/// Create a new project and insert it into the graph.
pub fn create_project(
    conn: &SparqlConnection,
    name: &str,
    description: Option<&str>,
) -> Result<Project, String> {
    let project_uri = format!("{}project/{}", Prefix::ZAKHOR, slugify(name));
    let safe_name = name.replace('\'', "\\'");
    let safe_desc = description
        .map(|d| d.replace('\'', "\\'"))
        .unwrap_or_default();

    let sparql = format!(
        r#"PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX zakhor: <{ns}>

INSERT DATA {{
  <{uri}> rdf:type zakhor:Project .
  <{uri}> rdfs:label "{name}"@en .
{desc_clause}}}"#,
        ns = Prefix::ZAKHOR,
        uri = project_uri,
        name = safe_name,
        desc_clause = if description.is_some() {
            format!("  <{}> rdfs:comment \"{}\"@en .", project_uri, safe_desc)
        } else {
            String::new()
        },
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
    let safe_entity = entity_uri.replace('>', "");
    let safe_project = project_uri.replace('>', "");

    let sparql = format!(
        r#"PREFIX zakhor: <{ns}>

INSERT DATA {{
  <{entity}> zakhor:belongsToProject <{project}> .
}}"#,
        ns = Prefix::ZAKHOR,
        entity = safe_entity,
        project = safe_project,
    );

    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Link to project failed: {e}"))?;
    Ok(())
}

/// List all projects.
pub fn list_projects(conn: &SparqlConnection) -> Result<Vec<Project>, String> {
    let sparql = format!(
        r#"PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX zakhor: <{ns}>

SELECT ?uri ?label ?comment WHERE {{
  ?uri rdf:type zakhor:Project .
  ?uri rdfs:label ?label .
  OPTIONAL {{ ?uri rdfs:comment ?comment . }}
}}
ORDER BY ?label"#,
        ns = Prefix::ZAKHOR,
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
}
