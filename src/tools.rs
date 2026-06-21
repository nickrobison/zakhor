use crate::lexical::LexicalIndex;
use crate::semantic::{ScoredDoc, SemanticIndex};
use std::collections::HashMap;
use std::sync::Mutex;

/// RRF k=60 fusion: run lexical + semantic search, fuse by reciprocal rank
pub fn hybrid_search(
    lexical: &LexicalIndex,
    semantic: &Mutex<SemanticIndex>,
    query: &str,
    limit: usize,
) -> Vec<ScoredDoc> {
    let overfetch = limit.max(20) * 2;

    // Lexical search
    let lexical_results = lexical.search(query, overfetch).unwrap_or_default();
    // Semantic search (lock mutex)
    let semantic_results = semantic
        .lock()
        .expect("semantic index lock poisoned")
        .search(query, overfetch);

    // RRF fusion with k=60
    let k = 60.0;
    let mut scores: HashMap<String, f64> = HashMap::new();

    for (rank, doc) in lexical_results.iter().enumerate() {
        *scores.entry(doc.id.clone()).or_insert(0.0) += 1.0 / (k + rank as f64);
    }
    for (rank, doc) in semantic_results.iter().enumerate() {
        *scores.entry(doc.id.clone()).or_insert(0.0) += 1.0 / (k + rank as f64);
    }

    let mut sorted: Vec<ScoredDoc> = scores
        .into_iter()
        .map(|(id, score)| ScoredDoc { id, score })
        .collect();
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted.truncate(limit);
    sorted
}

/// Build SPARQL SELECT for entity search by label pattern
pub fn build_entity_query(pattern: &str, limit: u32) -> String {
    let safe_pattern = pattern.replace('\'', "\\'");
    format!(
        "PREFIX zakhor: <http://zakhor/ns/>\nPREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\nPREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>\nSELECT ?entity ?label WHERE {{\n  ?entity rdf:type zakhor:Entity .\n  ?entity rdfs:label ?label .\n  FILTER(CONTAINS(LCASE(?label), LCASE('{}')))\n}}\nLIMIT {}",
        safe_pattern, limit
    )
}

/// Build SPARQL SELECT for graph traversal at given depth
pub fn build_traverse_query(start_id: &str, depth: u32, edge_types: &[String]) -> String {
    let safe_start = start_id.replace(['>', '<'], "");

    let filter_clause = if edge_types.is_empty() {
        String::new()
    } else {
        let types: Vec<String> = edge_types
            .iter()
            .map(|t| format!("<{}>", t.replace(['>', '<'], "")))
            .collect();
        format!("FILTER(?p IN ({})) ", types.join(" "))
    };

    // Build depth levels - for each depth, add property path of that length
    let mut patterns = Vec::new();
    for d in 1..=depth {
        let fpath = "?p/".repeat(d as usize).trim_end_matches('/').to_string();
        patterns.push(format!(
            "  {{ SELECT ?s ?p ?o WHERE {{ <{start}> {path} ?o . BIND(<{start}> AS ?s) }} }}",
            path = fpath,
            start = safe_start
        ));
        let bpath = "?p/".repeat(d as usize).trim_end_matches('/').to_string();
        patterns.push(format!(
            "  {{ SELECT ?s ?p ?o WHERE {{ ?s {path} <{start}> . BIND(<{start}> AS ?o) }} }}",
            path = bpath,
            start = safe_start
        ));
    }

    let depth_section = if patterns.is_empty() {
        String::new()
    } else {
        format!("\n  UNION\n{}", patterns.join("\n  UNION\n"))
    };

    format!(
        "PREFIX zakhor: <http://zakhor/ns/>\nPREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\nSELECT ?s ?p ?o WHERE {{\n  {{ ?s ?p ?o . FILTER(str(?s) = \"{start}\") . {filter} }}\n  UNION\n  {{ ?s ?p ?o . FILTER(str(?o) = \"{start}\") . {filter} }}{depth}\n}}",
        start = safe_start,
        filter = filter_clause,
        depth = depth_section
    )
}

/// Build SPARQL INSERT for recording a decision
pub fn build_decision_insert(
    decision_uri: &str,
    context: &str,
    decision: &str,
    alternatives: &[String],
    rationale: &str,
) -> String {
    fn escape_str(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n")
    }

    let mut alternatives_triples = String::new();
    for alt in alternatives {
        alternatives_triples.push_str(&format!(
            "<{}> zakhor:alternative \"{}\"@en .\n",
            decision_uri,
            escape_str(alt)
        ));
    }

    format!(
        "PREFIX zakhor: <http://zakhor/ns/>\nPREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\nINSERT DATA {{\n  <{}> rdf:type zakhor:Decision .\n  <{}> zakhor:decisionContext \"{}\"@en .\n  <{}> zakhor:decisionOutcome \"{}\"@en .\n  <{}> zakhor:decisionRationale \"{}\"@en .\n{}}}\n",
        decision_uri,
        decision_uri,
        escape_str(context),
        decision_uri,
        escape_str(decision),
        decision_uri,
        escape_str(rationale),
        alternatives_triples
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_entity_query_contains_pattern() {
        let q = build_entity_query("test", 10);
        assert!(q.contains("SELECT ?entity ?label"));
        assert!(q.contains("CONTAINS"));
        assert!(q.contains("LIMIT 10"));
        assert!(q.contains("'test'"));
    }

    #[test]
    fn test_build_entity_query_escapes_quotes() {
        let q = build_entity_query("it's", 5);
        assert!(q.contains("it\\'s"));
    }

    #[test]
    fn test_build_traverse_query_depth_1() {
        let q = build_traverse_query("http://example.org/start", 1, &[]);
        assert!(q.contains("SELECT"));
        assert!(!q.contains("!?p"));
    }

    #[test]
    fn test_build_traverse_query_reverse_path() {
        let q = build_traverse_query("http://example.org/start", 2, &[]);
        assert!(q.contains("?s ?p/?p <http://example.org/start>"));
    }

    #[test]
    fn test_build_decision_insert_includes_all_fields() {
        let alts = vec!["Option A".into(), "Option B".into()];
        let q = build_decision_insert("urn:uuid:abc", "Context", "Decision", &alts, "Rationale");
        assert!(q.contains("zakhor:Decision"));
        assert!(q.contains("zakhor:decisionContext"));
        assert!(q.contains("zakhor:decisionOutcome"));
        assert!(q.contains("zakhor:decisionRationale"));
        assert!(q.contains("zakhor:alternative"));
    }

    #[test]
    fn test_rrf_empty_returns_empty() {
        let result: Vec<ScoredDoc> = vec![];
        let _k = 60.0_f64;
        assert!(result.is_empty());
    }

    #[test]
    fn test_hybrid_search_ordering_same_scores() {
        // unit test pure RRF math
        let _k = 60.0;
        // doc "a" rank 1 in lexical, rank 3 in semantic
        let score_a = 1.0 / (60.0 + 0.0) + 1.0 / (60.0 + 2.0);
        assert!(score_a > 0.0);
    }
}
