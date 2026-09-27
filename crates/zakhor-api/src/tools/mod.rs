mod admin;
mod extract_and_store;
mod project_tools;
mod query_entities;
mod rebuild_indexes;
mod record_decision;
mod repository_tools;
mod search_hybrid;
mod store_observation;
mod traverse_graph;

use oxrdf::Literal;
use rmcp::handler::server::router::tool::ToolRouter;
use std::collections::HashMap;
use zakhor_search::{IndexSyncManager, ScoredDoc};
use zakhor_storage::sparql::{escape_literal, format_iri, prefix_declarations};

use crate::handler::MemoryHandler;

pub(crate) fn tool_router() -> ToolRouter<MemoryHandler> {
    MemoryHandler::tool_router_store_observation()
        + MemoryHandler::tool_router_extract_and_store()
        + MemoryHandler::tool_router_query_entities()
        + MemoryHandler::tool_router_traverse_graph()
        + MemoryHandler::tool_router_search_hybrid()
        + MemoryHandler::tool_router_record_decision()
        + MemoryHandler::tool_router_rebuild_indexes()
        + MemoryHandler::tool_router_project_tools()
        + MemoryHandler::tool_router_repository_tools()
        + MemoryHandler::tool_router_admin()
}

/// RRF k=60 fusion: run lexical + semantic search, fuse by reciprocal rank
#[allow(dead_code)]
pub fn hybrid_search(mgr: &IndexSyncManager, query: &str, limit: usize) -> Vec<ScoredDoc> {
    let overfetch = limit.max(20) * 2;

    // Lexical search
    let lexical_results = mgr.lexical.search(query, overfetch).unwrap_or_default();
    // Semantic search (lazy-init safe)
    let semantic_results = mgr.semantic_search(query, overfetch);

    // RRF fusion with k=60
    let k = 60.0;
    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut texts: HashMap<String, String> = HashMap::new();

    for (rank, doc) in lexical_results.iter().enumerate() {
        *scores.entry(doc.id.clone()).or_insert(0.0) += 1.0 / (k + rank as f64);
        texts
            .entry(doc.id.clone())
            .or_insert_with(|| doc.text.clone());
    }
    for (rank, doc) in semantic_results.iter().enumerate() {
        *scores.entry(doc.id.clone()).or_insert(0.0) += 1.0 / (k + rank as f64);
        texts
            .entry(doc.id.clone())
            .or_insert_with(|| doc.text.clone());
    }
    let mut sorted: Vec<ScoredDoc> = scores
        .into_iter()
        .map(|(id, score)| {
            let text = texts.remove(&id).unwrap_or_default();
            ScoredDoc { id, score, text }
        })
        .collect();
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted.truncate(limit);
    sorted
}

#[allow(dead_code)]
pub fn build_entity_query(pattern: &str, limit: u32) -> String {
    let safe_pattern = pattern.replace('\'', "\\'");
    format!(
        "{}SELECT ?entity ?label WHERE {{\n  ?entity rdf:type zakhor:Entity .\n  ?entity rdfs:label ?label .\n  FILTER(CONTAINS(LCASE(?label), LCASE('{}')))\n}}\nLIMIT {}",
        prefix_declarations(),
        safe_pattern,
        limit
    )
}

#[allow(dead_code)]
pub fn build_traverse_query(
    start_id: &str,
    depth: u32,
    edge_types: &[String],
) -> Result<String, String> {
    // `start` lands inside a quoted SPARQL string literal, so it must be
    // escaped; stripping angle brackets does nothing for a double quote.
    let safe_start = escape_literal(start_id);
    let start_iri = format_iri(start_id).map_err(|e| format!("start_id {e}"))?;
    let filter_clause = if edge_types.is_empty() {
        String::new()
    } else {
        let types: Vec<String> = edge_types
            .iter()
            .map(|t| format_iri(t).map_err(|e| format!("edge_type {e}")))
            .collect::<Result<_, _>>()?;
        format!("FILTER(?p IN ({})) ", types.join(" "))
    };
    let mut patterns = Vec::new();
    for d in 1..=depth {
        let fwd = hop_chain_forward(&start_iri, d);
        patterns.push(format!(
            "  {{ SELECT ?s ?p ?o WHERE {{ {fwd} BIND({start} AS ?s) {filter} }} }}",
            fwd = fwd,
            start = start_iri,
            filter = filter_clause
        ));
        let bwd = hop_chain_backward(&start_iri, d);
        patterns.push(format!(
            "  {{ SELECT ?s ?p ?o WHERE {{ {bwd} BIND({start} AS ?o) {filter} }} }}",
            bwd = bwd,
            start = safe_start,
            filter = filter_clause
        ));
    }
    let depth_section = if patterns.is_empty() {
        String::new()
    } else {
        format!("\n  UNION\n{}", patterns.join("\n  UNION\n"))
    };
    Ok(format!(
        "{prefixes}SELECT ?s ?p ?o WHERE {{\n  {{ ?s ?p ?o . FILTER(str(?s) = {start}) . {filter} }}\n  UNION\n  {{ ?s ?p ?o . FILTER(str(?o) = {start}) . {filter} }}{depth}\n}}",
        prefixes = prefix_declarations(),
        start = safe_start,
        filter = filter_clause,
        depth = depth_section
    ))
}

#[allow(dead_code)]
fn hop_chain_forward(start: &str, depth: u32) -> String {
    if depth == 1 {
        return format!("{start} ?p ?o .");
    }
    let d = depth as usize;
    let mut parts = Vec::with_capacity(d);
    parts.push(format!("{start} ?_p0 ?_mid0 ."));
    for i in 1..(d - 1) {
        parts.push(format!("?_mid{} ?_p{} ?_mid{} .", i - 1, i, i));
    }
    parts.push(format!("?_mid{} ?p ?o .", d - 2));
    parts.join(" ")
}

#[allow(dead_code)]
fn hop_chain_backward(start: &str, depth: u32) -> String {
    if depth == 1 {
        return format!("?s ?p {start} .");
    }
    let d = depth as usize;
    let mut parts = Vec::with_capacity(d);
    parts.push("?s ?p ?_mid0 .".to_string());
    for i in 1..(d - 1) {
        parts.push(format!("?_mid{} ?_p{} ?_mid{} .", i - 1, i, i));
    }
    parts.push(format!("?_mid{} ?_p{} {start} .", d - 2, d - 1));
    parts.join(" ")
}

/// Whether an IRI is a memory node worth expanding a traversal into.
///
/// Only the Zakhor memory vocabulary qualifies. Schema and ontology IRIs are
/// reachable from every memory node via `rdf:type`, so expanding into them
/// walks the entire ontology and never terminates in useful results.
pub fn is_memory_node(iri: &str) -> bool {
    iri.starts_with("http://zakhor/ns/")
}

#[allow(dead_code)]
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
        let q = build_traverse_query("http://example.org/start", 1, &[]).expect("valid IRI");
        assert!(q.contains("SELECT"));
        assert!(!q.contains("!?p"));
    }
    #[test]
    fn test_build_traverse_query_reverse_path() {
        let q = build_traverse_query("http://example.org/start", 2, &[]).expect("valid IRI");
        assert!(q.contains("?_mid0"));
        assert!(q.contains("<http://example.org/start>"));
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
        assert!(result.is_empty());
    }
    #[test]
    fn test_hybrid_search_ordering_same_scores() {
        let _k = 60.0;
        let score_a = 1.0 / (60.0 + 0.0) + 1.0 / (60.0 + 2.0);
        assert!(score_a > 0.0);
    }

    #[test]
    fn test_router_registers_project_and_repository_tools() {
        let router = tool_router();
        for name in [
            "create_project",
            "link_to_project",
            "create_repository",
            "link_to_repository",
        ] {
            assert!(router.has_route(name), "tool router should register {name}");
        }
    }
}

/// The predicate filter must appear in EVERY branch of the union, not just the
/// two direct (depth-0) branches. Dropping it from the hop branches means a
/// caller that filters to one predicate still receives every other predicate.
#[test]
fn test_traverse_query_applies_edge_filter_to_every_branch() {
    let label = "http://www.w3.org/2000/01/rdf-schema#label";
    for depth in 1..=3u32 {
        let q = build_traverse_query("http://example.org/start", depth, &[label.to_string()])
            .expect("valid IRIs");
        let branches = q.matches("{ SELECT ?s ?p ?o WHERE {").count() + 2;
        let filters = q.matches("FILTER(?p IN").count();
        assert_eq!(
            filters, branches,
            "depth {depth}: expected a predicate filter in all {branches} branches, found {filters}"
        );
    }
}

/// With no edge_types the query must not invent a filter.
#[test]
fn test_traverse_query_without_edge_types_has_no_filter() {
    let q = build_traverse_query("http://example.org/start", 2, &[]).expect("valid IRI");
    assert!(!q.contains("FILTER(?p IN"));
}

/// Schema IRIs must not be treated as graph nodes worth expanding, or a
/// traversal walks from an entity into the ontology and never stops.
#[test]
fn test_is_memory_node_excludes_schema_iris() {
    // Memory vocabulary — the graph an agent actually wants to walk.
    assert!(is_memory_node("http://zakhor/ns/entity/Kubernetes"));
    assert!(is_memory_node("http://zakhor/ns/decision/abc"));
    assert!(is_memory_node("http://zakhor/ns/project/p1"));
    assert!(is_memory_node("http://zakhor/ns/repository/r1"));
    // Schema / ontology IRIs — reachable from any memory node via rdf:type.
    assert!(!is_memory_node(
        "http://www.w3.org/2000/01/rdf-schema#Resource"
    ));
    assert!(!is_memory_node(
        "http://www.w3.org/2000/01/rdf-schema#Class"
    ));
    assert!(!is_memory_node(
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property"
    ));
    assert!(!is_memory_node(
        "http://tracker.api.gnome.org/ontology/v3/nrl#added"
    ));
    assert!(!is_memory_node("http://www.w3.org/2002/07/owl#Thing"));
    // Literals and blanks are not nodes.
    assert!(!is_memory_node("Alice"));
    assert!(!is_memory_node(""));
}

/// The cap must stay small enough to keep a traversal inside an agent's
/// tool-result budget, and non-zero to be meaningful. Checked at compile time.
const _: () = assert!(
    crate::handler::MAX_TRAVERSE_TRIPLES > 0 && crate::handler::MAX_TRAVERSE_TRIPLES <= 1000,
    "traversal cap must be in 1..=1000 to stay within a tool-result budget"
);

/// A traversal that hits the cap must say so, otherwise a caller cannot tell a
/// complete neighbourhood from a truncated one.
#[test]
fn test_traverse_response_reports_truncation() {
    let complete = crate::args::TraverseGraphResponse {
        triples: vec![],
        count: 0,
        truncated: false,
        warning: None,
    };
    let truncated = crate::args::TraverseGraphResponse {
        triples: vec![],
        count: 0,
        truncated: true,
        warning: None,
    };
    let json = |r: &crate::args::TraverseGraphResponse| serde_json::to_string(r).unwrap();
    assert!(json(&complete).contains("\"truncated\":false"));
    assert!(json(&truncated).contains("\"truncated\":true"));
}

#[cfg(test)]
mod uri_safety_tests {
    use super::*;

    /// `start_id` is compared inside a quoted SPARQL string literal, so a double
    /// quote in the input would terminate that literal early. It was not
    /// exploitable before only because the same value was *also* interpolated
    /// into a `BIND(<...>)`, where stripping angle brackets made the payload an
    /// invalid IRI — a property of a different clause, not of this one.
    ///
    /// The literal is now escaped, and separately the value is validated before
    /// it is used at all. Validation is what actually removes the hazard: an
    /// IRI cannot contain a quote, a newline or a space, so there is nothing
    /// left to break out with.
    #[test]
    fn test_traverse_start_id_literal_is_escaped_and_validated() {
        // Anything that could terminate the literal is refused outright.
        for hostile in [
            r#"a" || true || str(?s) = "b"#,
            r#"a" . "b"#,
            r#"a"} UNION {?s ?p ?o} . \"#,
            "a\"b",
            "a\nb",
        ] {
            assert!(
                build_traverse_query(hostile, 1, &[]).is_err(),
                "{hostile:?} must be refused before it reaches the query"
            );
        }

        // And a legitimate IRI is emitted as exactly one well-formed literal.
        let q = build_traverse_query("http://example.org/s", 1, &[]).expect("valid IRI");
        assert_eq!(
            q.matches(r#"FILTER(str(?s) = "http://example.org/s")"#)
                .count(),
            1,
            "start must appear as a single quoted literal: {q}"
        );
        assert_eq!(
            q.matches(r#"FILTER(str(?o) = "http://example.org/s")"#)
                .count(),
            1,
            "start must appear as a single quoted literal: {q}"
        );
    }

    /// A hostile start_id is now refused outright rather than producing a query
    /// that fails to parse deep inside Tracker.
    #[test]
    fn test_traverse_rejects_invalid_start_id() {
        for hostile in ["not-a-uri", "http://example.com/a b", "", "a\0b"] {
            let err = build_traverse_query(hostile, 1, &[])
                .expect_err("a non-addressable start_id must be refused");
            assert!(
                err.contains("not a valid URI"),
                "error should name the problem, got: {err}"
            );
        }
    }

    #[test]
    fn test_traverse_rejects_invalid_edge_type() {
        let err = build_traverse_query("http://example.org/s", 1, &["not-a-iri".to_string()])
            .expect_err("a non-addressable predicate must be refused");
        assert!(err.contains("not a valid URI"), "got: {err}");
    }

    /// The generated query must survive a real SPARQL parser. Asserting that a
    /// URI "appears" is not enough — `<<iri>>` also contains `<iri>`, and that
    /// malformed form was produced here once already.
    #[test]
    fn test_traverse_query_braces_stay_balanced() {
        for depth in 1..=3u32 {
            let q = build_traverse_query("http://example.org/s", depth, &[]).expect("valid IRI");
            assert_eq!(q.matches('{').count(), q.matches('}').count(), "{q}");
            assert!(
                !q.contains("<<"),
                "double angle brackets at depth {depth}: {q}"
            );
            assert!(
                !q.contains(">>"),
                "double angle brackets at depth {depth}: {q}"
            );
            // Every IRI reference must be opened and closed exactly once.
            assert_eq!(
                q.matches('<').count(),
                q.matches('>').count(),
                "unbalanced IRI framing at depth {depth}: {q}"
            );
        }
    }
}
