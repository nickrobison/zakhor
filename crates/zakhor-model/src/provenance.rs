#[allow(unused_imports)]
use gio::Cancellable;
use std::collections::HashMap;
#[allow(unused_imports)]
use tracker::prelude::{SparqlConnectionExtManual, SparqlCursorExtManual};
use tracker::SparqlConnection;
use zakhor_common::vocab::NAMED_GRAPH_PREFIX as GRAPH_PREFIX;
use zakhor_storage::sparql::{self as storage_sparql};

/// Build a named graph URI string for the given observation UUID.
pub fn graph_uri(uuid: &str) -> String {
    format!("{}{}", GRAPH_PREFIX, uuid)
}

/// Tracks provenance of observations using named graphs.
///
/// Each observation is stored as a set of triples in a named graph
/// identified by `zakhor:graph/{observation-uuid}`.
pub struct ProvenanceTracker {
    /// Map of graph_uri → triples (subject, predicate, object).
    graphs: HashMap<String, Vec<(String, String, String)>>,
}

impl ProvenanceTracker {
    pub fn new() -> Self {
        Self {
            graphs: HashMap::new(),
        }
    }

    pub fn add_observation(&mut self, uuid: &str, triples: Vec<(String, String, String)>) {
        let graph_name = graph_uri(uuid);
        self.graphs.entry(graph_name).or_default().extend(triples);
    }

    pub fn get_observation_graph(&self, uuid: &str) -> Vec<(String, String, String)> {
        let graph_name = graph_uri(uuid);
        self.graphs.get(&graph_name).cloned().unwrap_or_default()
    }

    pub fn all_observations(&self) -> Vec<String> {
        let mut uuids: Vec<String> = Vec::new();
        for graph_name in self.graphs.keys() {
            if let Some(uuid) = graph_name.strip_prefix(GRAPH_PREFIX) {
                let uuid = uuid.to_string();
                if !uuids.contains(&uuid) {
                    uuids.push(uuid);
                }
            }
        }
        uuids
    }

    pub fn contains_observation(&self, uuid: &str) -> bool {
        let graph_name = graph_uri(uuid);
        self.graphs.contains_key(&graph_name)
    }

    pub fn clear(&mut self) {
        self.graphs.clear();
    }

    /// Flush all tracked named graphs to the SPARQL triplestore.
    ///
    /// Each observation graph is written as an `INSERT DATA { GRAPH <uri> { ... } }`
    /// statement.  The in-memory tracker is **not** cleared — call `.clear()` if
    /// you want to reclaim memory after flushing.
    pub fn flush_to_sparql(&self, conn: &SparqlConnection) -> Result<u64, String> {
        let mut total: u64 = 0;
        for uuid in self.all_observations() {
            let triples = self.get_observation_graph(&uuid);
            if triples.is_empty() {
                continue;
            }
            let sparql = build_named_graph_insert(&uuid, &triples);
            conn.update(&sparql, None::<&Cancellable>)
                .map_err(|e| format!("Failed to flush graph {}: {}", uuid, e))?;
            total += triples.len() as u64;
        }
        Ok(total)
    }
}

impl Default for ProvenanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// SPARQL helpers
// ---------------------------------------------------------------------------

/// Build an `INSERT DATA { GRAPH <uri> { ... } }` statement for a named graph.
///
/// Each triple `(s, p, o)` is inserted into the named graph `zakhor:graph/{uuid}`.
fn build_named_graph_insert(uuid: &str, triples: &[(String, String, String)]) -> String {
    let mut sparql = String::with_capacity(512 + triples.len() * 128);
    sparql.push_str(&storage_sparql::prefix_declarations());
    sparql.push_str("INSERT DATA {\n");
    sparql.push_str(&format!("  GRAPH <{}> {{\n", graph_uri(uuid)));
    for (s, p, o) in triples {
        sparql.push_str(&format!("    <{}> <{}> <{}> .\n", s, p, o));
    }
    sparql.push_str("  }\n");
    sparql.push_str("}\n");
    sparql
}

/// Query all triples in a named graph from the SPARQL store.
///
/// Returns `(subject, predicate, object)` tuples.
pub fn query_named_graph(
    conn: &SparqlConnection,
    uuid: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let graph = graph_uri(uuid);
    let sparql = format!(
        "{}SELECT ?s ?p ?o WHERE {{ GRAPH <{}> {{ ?s ?p ?o }} }}",
        storage_sparql::prefix_declarations(),
        graph,
    );
    let cursor = conn
        .query(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Named graph query failed: {}", e))?;

    let mut results = Vec::new();
    while cursor
        .next(None::<&Cancellable>)
        .map_err(|e| format!("Cursor error: {}", e))?
    {
        let s = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let p = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        let o = cursor.string(2).map(|s| s.to_string()).unwrap_or_default();
        results.push((s, p, o));
    }
    Ok(results)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_uri_format() {
        let uri = graph_uri("abc-123");
        assert_eq!(uri, "http://zakhor/ns/graph/abc-123");
    }

    #[test]
    fn test_build_named_graph_insert_basic() {
        let triples = vec![
            ("urn:s1".into(), "urn:p1".into(), "urn:o1".into()),
            ("urn:s2".into(), "urn:p2".into(), "urn:o2".into()),
        ];
        let sparql = build_named_graph_insert("test-uuid", &triples);
        assert!(sparql.starts_with("PREFIX"), "should start with PREFIX");
        assert!(sparql.contains("INSERT DATA"), "should have INSERT DATA");
        assert!(
            sparql.contains("GRAPH <http://zakhor/ns/graph/test-uuid>"),
            "should use correct graph URI"
        );
        assert!(sparql.contains("<urn:s1>"), "should contain first subject");
        assert!(sparql.contains("<urn:o2>"), "should contain second object");
        let opens = sparql.matches('{').count();
        let closes = sparql.matches('}').count();
        assert_eq!(opens, closes, "braces should be balanced");
    }

    #[test]
    fn test_build_named_graph_insert_empty() {
        let sparql = build_named_graph_insert("empty-uuid", &[]);
        assert!(sparql.contains("GRAPH <http://zakhor/ns/graph/empty-uuid>"));
        // Should still have the GRAPH block even if empty
        assert!(sparql.contains("{\n  }"));
    }

    #[test]
    fn test_new_tracker_empty() {
        let tracker = ProvenanceTracker::new();
        assert!(tracker.all_observations().is_empty());
        assert!(!tracker.contains_observation("any-uuid"));
    }

    #[test]
    fn test_add_and_get_observation() {
        let mut tracker = ProvenanceTracker::new();
        let triples = vec![
            ("urn:s1".into(), "urn:p1".into(), "urn:o1".into()),
            ("urn:s2".into(), "urn:p2".into(), "urn:o2".into()),
        ];
        tracker.add_observation("obs-1", triples);

        let result = tracker.get_observation_graph("obs-1");
        assert_eq!(result.len(), 2);
        assert!(result.contains(&("urn:s1".into(), "urn:p1".into(), "urn:o1".into())));
    }

    #[test]
    fn test_named_graph_prefix_constant() {
        assert_eq!(GRAPH_PREFIX, "http://zakhor/ns/graph/");
    }
}
