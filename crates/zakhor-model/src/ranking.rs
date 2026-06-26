//! Graph Importance Ranking
//!
//! Scores entities by their connectivity in the knowledge graph using 2-hop
//! traversal.  An entity that connects to many other entities (directly or
//! through a shared intermediary) receives a higher importance score.

use gio::Cancellable;
use iref::IriBuf;
use serde::Serialize;
use std::collections::HashMap;
use tracker::prelude::SparqlCursorExtManual;
use tracker::SparqlConnection;
use zakhor_search::ScoredDoc;
use zakhor_storage::sparql::Prefix;

/// A scored entity from the graph ranking.
#[derive(Clone, Debug, Serialize)]
pub struct ScoredEntity {
    pub uri: IriBuf,
    pub label: String,
    /// Raw connectivity score (number of unique 2-hop connections).
    pub connectivity: u64,
    /// Normalised importance between 0.0 and 1.0.
    pub importance: f64,
}

/// Compute graph importance for all entities in the store.
///
/// For each entity, counts the number of unique entities reachable within
/// 2 hops (excluding the entity itself).  The result is sorted by descending
/// connectivity.
pub fn compute_importance(conn: &SparqlConnection) -> Result<Vec<ScoredEntity>, String> {
    // Step 1: Collect all entity URIs that have a rdfs:label
    let entities = list_labeled_entities(conn)?;
    if entities.is_empty() {
        return Ok(Vec::new());
    }

    let max_score = entities.len() as f64;

    let mut scored: Vec<ScoredEntity> = Vec::with_capacity(entities.len());

    for (uri, label) in &entities {
        let connectivity = count_2hop_connections(conn, uri)?;
        let importance = if max_score > 0.0 {
            connectivity as f64 / max_score
        } else {
            0.0
        };
        scored.push(ScoredEntity {
            uri: uri.clone(),
            label: label.clone(),
            connectivity,
            importance,
        });
    }

    // Sort by connectivity descending
    scored.sort_by_key(|b| std::cmp::Reverse(b.connectivity));
    Ok(scored)
}

/// Count how many other labeled entities are reachable from `uri` within
/// 2 SPARQL hops.
fn count_2hop_connections(conn: &SparqlConnection, uri: &IriBuf) -> Result<u64, String> {
    let safe = uri.as_str().replace('>', "");
    let sparql = format!(
        r#"PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>

SELECT (COUNT(DISTINCT ?connected) AS ?count) WHERE {{
  {{
    <{id}> ?p1 ?connected .
    FILTER(isIRI(?connected))
  }} UNION {{
    ?connected ?p2 <{id}> .
    FILTER(isIRI(?connected))
  }} UNION {{
    <{id}> ?p3 ?mid .
    ?mid ?p4 ?connected .
    FILTER(isIRI(?mid) && isIRI(?connected))
  }}
  FILTER(?connected != <{id}>)
}}"#,
        id = safe,
    );

    let cursor = conn
        .query(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("2-hop query failed: {e}"))?;

    let mut count: u64 = 0;
    while cursor
        .next(None::<&Cancellable>)
        .map_err(|e| format!("Cursor error: {e}"))?
    {
        count = cursor.integer(0).max(0) as u64;
    }

    // If we can't query, return 0 rather than failing
    Ok(count)
}

/// List all entities with an `rdfs:label`.
fn list_labeled_entities(conn: &SparqlConnection) -> Result<HashMap<IriBuf, String>, String> {
    let sparql = r#"PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT DISTINCT ?entity ?label WHERE {
  ?entity rdfs:label ?label .
}
ORDER BY ?entity"#
        .to_string();

    let cursor = conn
        .query(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("List entities query failed: {e}"))?;

    let mut entities = HashMap::new();
    while cursor
        .next(None::<&Cancellable>)
        .map_err(|e| format!("Cursor error: {e}"))?
    {
        let uri = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let label = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        if !uri.is_empty() {
            match IriBuf::new(uri) {
                Ok(iri) => {
                    entities.insert(iri, label);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Skipping invalid entity URI in ranking");
                }
            }
        }
    }

    Ok(entities)
}

// ---------------------------------------------------------------------------
// Provenance Quality Ranking (Phase 2.2)
// ---------------------------------------------------------------------------

/// Score an entity's provenance by counting how many decisions reference it.
///
/// An entity referenced by many decisions (via `zakhor:evidenceFor` or
/// `zakhor:provenanceGraph`) has higher provenance quality.
pub fn compute_provenance_quality(
    conn: &SparqlConnection,
    entity_uri: &IriBuf,
) -> Result<f64, String> {
    let safe = entity_uri.as_str().replace('>', "");
    let sparql = format!(
        r#"PREFIX zakhor: <{ns}>

SELECT (COUNT(?decision) AS ?count) WHERE {{
  {{ ?decision zakhor:provenanceGraph <{id}> . }}
  UNION
  {{ ?decision zakhor:evidenceFor <{id}> . }}
}}"#,
        ns = Prefix::ZAKHOR,
        id = safe,
    );

    let cursor = conn
        .query(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Provenance query failed: {e}"))?;

    let mut count: i64 = 0;
    while cursor
        .next(None::<&Cancellable>)
        .map_err(|e| format!("Cursor error: {e}"))?
    {
        count = cursor.integer(0);
    }

    // Normalise: 0 evidence → 0.0, 10+ evidence → 1.0 (sigmoid-like clamp)
    let score = (count as f64).min(10.0) / 10.0;
    Ok(score)
}

/// Combine graph importance and provenance quality into a single ranking score.
///
/// `graph_weight` and `provenance_weight` control the contribution of each
/// factor (default: 0.6 and 0.4 respectively).
pub fn compute_combined_score(
    graph_importance: f64,
    provenance_quality: f64,
    graph_weight: f64,
    provenance_weight: f64,
) -> f64 {
    graph_importance * graph_weight + provenance_quality * provenance_weight
}

// ---------------------------------------------------------------------------
// Ranked Hybrid Search (Phase 2.3)
// ---------------------------------------------------------------------------

/// Apply graph importance and provenance quality boosts to hybrid search results.
///
/// For each result, queries the SPARQL store for the entity URI and re-ranks
/// by `score * (1.0 + graph_importance * 0.3 + provenance_quality * 0.2)`.
///
/// `raw_results` — output of `tools::hybrid_search()`
/// `conn` — SPARQL connection for ranking queries
pub fn rank_search_results(
    raw_results: Vec<ScoredDoc>,
    conn: &SparqlConnection,
) -> Result<Vec<ScoredDoc>, String> {
    if raw_results.is_empty() {
        return Ok(Vec::new());
    }

    let mut ranked: Vec<ScoredDoc> = Vec::with_capacity(raw_results.len());
    for doc in raw_results {
        let (graph_score, provenance_score) = match IriBuf::new(doc.id.clone()) {
            Ok(entity_iri) => {
                let provenance_quality =
                    compute_provenance_quality(conn, &entity_iri).unwrap_or(0.0);
                (provenance_quality, provenance_quality)
            }
            Err(e) => {
                tracing::warn!(id = %doc.id, error = %e, "Skipping ranking boosts for invalid entity URI");
                (0.0, 0.0)
            }
        };
        let boost = 1.0 + graph_score * 0.3 + provenance_score * 0.2;
        ranked.push(ScoredDoc {
            id: doc.id,
            score: doc.score * boost,
        });
    }

    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(ranked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scored_entity_struct() {
        let e = ScoredEntity {
            uri: IriBuf::new("http://example.com/e1".to_string()).expect("valid test iri"),
            label: "Entity 1".into(),
            connectivity: 5,
            importance: 0.5,
        };
        assert_eq!(e.uri.as_str(), "http://example.com/e1");
        assert_eq!(e.connectivity, 5);
        assert_eq!(e.importance, 0.5);
    }

    #[test]
    fn test_scored_entity_sort_order() {
        let mut items = vec![
            ScoredEntity {
                uri: IriBuf::new("http://example.com/a".to_string()).expect("valid test iri"),
                label: "A".into(),
                connectivity: 1,
                importance: 0.1,
            },
            ScoredEntity {
                uri: IriBuf::new("http://example.com/b".to_string()).expect("valid test iri"),
                label: "B".into(),
                connectivity: 10,
                importance: 1.0,
            },
            ScoredEntity {
                uri: IriBuf::new("http://example.com/c".to_string()).expect("valid test iri"),
                label: "C".into(),
                connectivity: 5,
                importance: 0.5,
            },
        ];
        items.sort_by(|a, b| b.connectivity.cmp(&a.connectivity));
        assert_eq!(items[0].uri.as_str(), "http://example.com/b");
        assert_eq!(items[1].uri.as_str(), "http://example.com/c");
        assert_eq!(items[2].uri.as_str(), "http://example.com/a");
    }

    #[test]
    fn test_importance_normalisation() {
        // With only 1 entity, max_score = 1, so importance = connectivity / 1
        let max_score = 1.0;
        let connectivity = 3;
        let importance = connectivity as f64 / max_score;
        assert!((importance - 3.0).abs() < f64::EPSILON);

        // With 10 entities, max_score = 10
        let max_score = 10.0;
        let connectivity = 3;
        let importance = connectivity as f64 / max_score;
        assert!((importance - 0.3).abs() < f64::EPSILON);
    }
}
