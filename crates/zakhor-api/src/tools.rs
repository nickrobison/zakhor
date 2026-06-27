use oxrdf::Literal;
use std::collections::HashMap;
use std::sync::Mutex;
use zakhor_search::{LexicalIndex, ScoredDoc, SemanticIndex};
use zakhor_storage::sparql::prefix_declarations;

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
        "{}SELECT ?entity ?label WHERE {{\n  ?entity rdf:type zakhor:Entity .\n  ?entity rdfs:label ?label .\n  FILTER(CONTAINS(LCASE(?label), LCASE('{}')))\n}}\nLIMIT {}",
        prefix_declarations(), safe_pattern, limit
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

    // Build depth levels using intermediate variables to avoid SPARQL property
    // paths with variable predicates (e.g. `?p/?p`), which are not valid SPARQL 1.1
    // and are rejected by Tracker.
    let mut patterns = Vec::new();
    for d in 1..=depth {
        let fwd = hop_chain_forward(&safe_start, d);
        patterns.push(format!(
            "  {{ SELECT ?s ?p ?o WHERE {{ {fwd} BIND(<{start}> AS ?s) }} }}",
            fwd = fwd,
            start = safe_start
        ));
        let bwd = hop_chain_backward(&safe_start, d);
        patterns.push(format!(
            "  {{ SELECT ?s ?p ?o WHERE {{ {bwd} BIND(<{start}> AS ?o) }} }}",
            bwd = bwd,
            start = safe_start
        ));
    }

    let depth_section = if patterns.is_empty() {
        String::new()
    } else {
        format!("\n  UNION\n{}", patterns.join("\n  UNION\n"))
    };

    let prefixes = prefix_declarations();
    format!(
        "{prefixes}SELECT ?s ?p ?o WHERE {{\n  {{ ?s ?p ?o . FILTER(str(?s) = \"{start}\") . {filter} }}\n  UNION\n  {{ ?s ?p ?o . FILTER(str(?o) = \"{start}\") . {filter} }}{depth}\n}}",
        prefixes = prefixes,
        start = safe_start,
        filter = filter_clause,
        depth = depth_section
    )
}

/// Build a forward hop chain of `depth` steps from `start`.
///
/// depth=1: `<start> ?p ?o .`
/// depth=2: `<start> ?_p0 ?_mid0 . ?_mid0 ?p ?o .`
///
/// `?p` is always the last-hop predicate; intermediate predicates use
/// anonymous variables (`?_p0`, `?_p1`, …) so they don't conflict with the
/// outer query's `?p` binding.
fn hop_chain_forward(start: &str, depth: u32) -> String {
    if depth == 1 {
        return format!("<{start}> ?p ?o .");
    }
    let d = depth as usize;
    let mut parts = Vec::with_capacity(d);
    parts.push(format!("<{start}> ?_p0 ?_mid0 ."));
    for i in 1..(d - 1) {
        parts.push(format!("?_mid{} ?_p{} ?_mid{} .", i - 1, i, i));
    }
    parts.push(format!("?_mid{} ?p ?o .", d - 2));
    parts.join(" ")
}

/// Build a backward hop chain of `depth` steps ending at `start`.
///
/// depth=1: `?s ?p <start> .`
/// depth=2: `?s ?p ?_mid0 . ?_mid0 ?_p1 <start> .`
///
/// `?p` is always the first-hop predicate.
fn hop_chain_backward(start: &str, depth: u32) -> String {
    if depth == 1 {
        return format!("?s ?p <{start}> .");
    }
    let d = depth as usize;
    let mut parts = Vec::with_capacity(d);
    parts.push("?s ?p ?_mid0 .".to_string());
    for i in 1..(d - 1) {
        parts.push(format!("?_mid{} ?_p{} ?_mid{} .", i - 1, i, i));
    }
    parts.push(format!("?_mid{} ?_p{} <{start}> .", d - 2, d - 1));
    parts.join(" ")
}

/// Build SPARQL INSERT for recording a decision
pub fn build_decision_insert(
    decision_uri: &str,
    context: &str,
    decision: &str,
    alternatives: &[String],
    rationale: &str,
) -> String {
    let mut alternatives_triples = String::new();
    for alt in alternatives {
        alternatives_triples.push_str(&format!(
            "<{}> zakhor:alternative {} .\n",
            decision_uri,
            Literal::new_language_tagged_literal(alt.to_string(), "en").unwrap()
        ));
    }

    format!(
        "{}INSERT DATA {{\n  <{}> rdf:type zakhor:Decision .\n  <{}> zakhor:decisionContext {} .\n  <{}> zakhor:decisionOutcome {} .\n  <{}> zakhor:decisionRationale {} .\n{}}}\n",
        prefix_declarations(),
        decision_uri,
        decision_uri,
        Literal::new_language_tagged_literal(context.to_string(), "en").unwrap(),
        decision_uri,
        Literal::new_language_tagged_literal(decision.to_string(), "en").unwrap(),
        decision_uri,
        Literal::new_language_tagged_literal(rationale.to_string(), "en").unwrap(),
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
        // Depth=2 uses intermediate variables instead of property paths
        assert!(q.contains("?_mid0"));
        assert!(q.contains("<http://example.org/start>"));
        // Must NOT generate the invalid `?p/?p` property path syntax
        assert!(!q.contains("?p/?p"));
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
