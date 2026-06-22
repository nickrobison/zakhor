//! Code Indexing (Phase 2.5 / 2.6)
//!
//! Tier 1: Containers auto-create — for each ingested code file, a
//!   `code:Container` entity is created in the knowledge graph.
//!
//! Tier 2: Symbols link-only — function/class/interface symbols are
//!   created and linked to their container but the graph does *not*
//!   store the full AST (the code host / file system remains the
//!   source of truth).

use gio::Cancellable;
use tracker::SparqlConnection;
use tracker::prelude::{SparqlConnectionExtManual, SparqlCursorExtManual};

use crate::sparql::Prefix;

/// A code container (file, module, package, repository).
#[derive(Clone, Debug)]
pub struct CodeContainer {
    pub uri: String,
    pub path: String,
    pub language: Option<String>,
}

/// A code symbol (function, class, interface, type).
#[derive(Clone, Debug)]
pub struct CodeSymbol {
    pub uri: String,
    pub name: String,
    pub kind: String, // function | class | interface | type
    pub container_uri: String,
    pub line_start: Option<u32>,
}

/// Create a code container for a file/repository.
///
/// Tier 1: inserts a `zakhor:CodeContainer` with its path and language.
pub fn create_container(
    conn: &SparqlConnection,
    path: &str,
    language: Option<&str>,
) -> Result<CodeContainer, String> {
    let container_uri = format!("{}code/container/{}", Prefix::ZAKHOR, path_hash(path));
    let safe_path = path.replace('\'', "\\'");
    let lang_clause = language
        .map(|l| format!("  <{}> zakhor:codeLanguage \"{}\"@en .", container_uri, l))
        .unwrap_or_default();

    let sparql = format!(
        r#"PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX zakhor: <{ns}>

INSERT DATA {{
  <{uri}> rdf:type zakhor:CodeContainer .
  <{uri}> zakhor:codeLocation "{path}"@en .
{lang_clause}}}"#,
        ns = Prefix::ZAKHOR,
        uri = container_uri,
        path = safe_path,
        lang_clause = lang_clause,
    );

    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Create container failed: {e}"))?;

    Ok(CodeContainer {
        uri: container_uri,
        path: path.to_string(),
        language: language.map(String::from),
    })
}

/// Create a code symbol and link it to its container.
///
/// Tier 2: the symbol is stored as a `zakhor:CodeSymbol` with its name,
/// kind, and container reference.  Full AST content is NOT duplicated
/// into the graph.
pub fn create_symbol(
    conn: &SparqlConnection,
    name: &str,
    kind: &str,
    container_uri: &str,
    line_start: Option<u32>,
) -> Result<CodeSymbol, String> {
    let symbol_uri = format!(
        "{}code/symbol/{}/{}",
        Prefix::ZAKHOR,
        slugify(name),
        hex_hash(name)
    );
    let safe_name = name.replace('\'', "\\'");
    let line_clause = line_start
        .map(|l| format!("  <{}> zakhor:codeLineStart {} .", symbol_uri, l))
        .unwrap_or_default();

    let sparql = format!(
        r#"PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX zakhor: <{ns}>

INSERT DATA {{
  <{uri}> rdf:type zakhor:CodeSymbol .
  <{uri}> rdfs:label "{name}"@en .
  <{uri}> zakhor:codeSymbolKind "{kind}"@en .
  <{uri}> zakhor:codeLocation <{container}> .
{line_clause}}}"#,
        ns = Prefix::ZAKHOR,
        uri = symbol_uri,
        name = safe_name,
        kind = kind,
        container = container_uri,
        line_clause = line_clause,
    );

    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Create symbol failed: {e}"))?;

    Ok(CodeSymbol {
        uri: symbol_uri,
        name: name.to_string(),
        kind: kind.to_string(),
        container_uri: container_uri.to_string(),
        line_start,
    })
}

/// List all containers, optionally filtered by language.
pub fn list_containers(
    conn: &SparqlConnection,
    language: Option<&str>,
) -> Result<Vec<CodeContainer>, String> {
    let filter = language
        .map(|l| format!("FILTER(?lang = \"{}\"@en)", l.replace('\'', "\\'")))
        .unwrap_or_default();

    let sparql = format!(
        r#"PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX zakhor: <{ns}>

SELECT ?uri ?path ?lang WHERE {{
  ?uri rdf:type zakhor:CodeContainer .
  ?uri zakhor:codeLocation ?path .
  OPTIONAL {{ ?uri zakhor:codeLanguage ?lang . }}
  {filter}
}}
ORDER BY ?path"#,
        ns = Prefix::ZAKHOR,
        filter = filter,
    );

    let cursor = conn
        .query(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("List containers failed: {e}"))?;

    let mut containers = Vec::new();
    while cursor
        .next(None::<&Cancellable>)
        .map_err(|e| format!("Cursor error: {e}"))?
    {
        let uri = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let path = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        let lang = cursor.string(2).map(|s| s.to_string());
        containers.push(CodeContainer {
            uri,
            path,
            language: lang.filter(|s| !s.is_empty()),
        });
    }

    Ok(containers)
}

fn path_hash(path: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn hex_hash(s: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:08x}", hasher.finish())
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
    fn test_code_container_struct() {
        let c = CodeContainer {
            uri: "http://zakhor/ns/code/container/abc".into(),
            path: "src/main.rs".into(),
            language: Some("rust".into()),
        };
        assert_eq!(c.path, "src/main.rs");
    }

    #[test]
    fn test_code_symbol_struct() {
        let s = CodeSymbol {
            uri: "http://zakhor/ns/code/symbol/foo".into(),
            name: "foo".into(),
            kind: "function".into(),
            container_uri: "http://zakhor/ns/code/container/abc".into(),
            line_start: Some(42),
        };
        assert_eq!(s.name, "foo");
        assert_eq!(s.line_start, Some(42));
    }

    #[test]
    fn test_path_hash_uniqueness() {
        let h1 = path_hash("src/main.rs");
        let h2 = path_hash("src/lib.rs");
        let h3 = path_hash("src/main.rs");
        assert_ne!(h1, h2);
        assert_eq!(h1, h3);
        assert_eq!(h1.len(), 16);
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("MyFunction"), "myfunction");
        assert_eq!(slugify("parse_input"), "parse-input");
    }
}
