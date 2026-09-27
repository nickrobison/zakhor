//! Shared SPARQL projection used by both index rebuilds.
//!
//! The lexical (BM25) and semantic (embedding) indexes must agree on *what* is
//! indexable, otherwise a rebuild makes the two disagree and hybrid fusion
//! ranks against mismatched corpora. Both therefore read from here.
//!
//! Observations (`nie:InformationElement`) carry their own text. Decisions do
//! not: a decision stores its prose across several `zakhor:` predicates and has
//! no `nie:identifier` or `nie:plainTextContent`, so it is invisible to a plain
//! `nie:InformationElement` query. Without an explicit projection a recorded
//! decision is write-only — it can never be found again.

use std::collections::HashMap;

use tracker::SparqlConnection;
use tracker::prelude::SparqlCursorExtManual;

use zakhor_common::error::{ZakhorError, ZakhorResult};
use zakhor_common::vocab;

/// A single indexable document: its stable id and the text to index for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexableDoc {
    pub id: String,
    pub text: String,
}

const OBSERVATIONS_QUERY: &str = "\
    PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>\n\
    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
    SELECT ?identifier ?text WHERE {\n\
        ?id rdf:type nie:InformationElement ;\n\
            nie:identifier ?identifier ;\n\
            nie:plainTextContent ?text .\n\
    }";

/// Decisions, one row per decision. The narrative fields are single-valued, so
/// they project cleanly; alternatives are multi-valued and fetched separately.
fn decisions_query() -> String {
    format!(
        "\
    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
    SELECT ?id ?context ?outcome ?rationale WHERE {{\n\
        ?id rdf:type <{decision}> ;\n\
            <{context}> ?context ;\n\
            <{outcome}> ?outcome ;\n\
            <{rationale}> ?rationale .\n\
    }}",
        decision = vocab::decision_iri(),
        context = vocab::decision_context_iri(),
        outcome = vocab::decision_outcome_iri(),
        rationale = vocab::decision_rationale_iri(),
    )
}

fn alternatives_query() -> String {
    format!(
        "\
    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
    SELECT ?id ?alternative WHERE {{\n\
        ?id rdf:type <{decision}> ;\n\
            <{alternative}> ?alternative .\n\
    }}",
        decision = vocab::decision_iri(),
        alternative = vocab::decision_alternative_iri(),
    )
}

fn fetch_observations(conn: &SparqlConnection) -> ZakhorResult<Vec<IndexableDoc>> {
    let cursor = conn
        .query(OBSERVATIONS_QUERY, None::<&gio::Cancellable>)
        .map_err(|e| ZakhorError::Database(format!("observation query failed: {e}")))?;

    let mut docs = Vec::new();
    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ZakhorError::Database(format!("observation cursor failed: {e}")))?
    {
        let Some(identifier) = cursor.string(0) else {
            continue;
        };
        let Some(text) = cursor.string(1) else {
            continue;
        };
        docs.push(IndexableDoc {
            id: identifier.to_string(),
            text: text.to_string(),
        });
    }
    Ok(docs)
}

fn fetch_decisions(conn: &SparqlConnection) -> ZakhorResult<Vec<IndexableDoc>> {
    // Alternatives are keyed by decision URI and merged back before composing.
    let mut alternatives: HashMap<String, Vec<String>> = HashMap::new();
    let cursor = conn
        .query(&alternatives_query(), None::<&gio::Cancellable>)
        .map_err(|e| ZakhorError::Database(format!("decision alternatives query failed: {e}")))?;
    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ZakhorError::Database(format!("alternatives cursor failed: {e}")))?
    {
        if let (Some(id), Some(alternative)) = (cursor.string(0), cursor.string(1)) {
            alternatives
                .entry(id.to_string())
                .or_default()
                .push(alternative.to_string());
        }
    }

    let cursor = conn
        .query(&decisions_query(), None::<&gio::Cancellable>)
        .map_err(|e| ZakhorError::Database(format!("decision query failed: {e}")))?;

    let mut docs = Vec::new();
    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ZakhorError::Database(format!("decision cursor failed: {e}")))?
    {
        let Some(id) = cursor.string(0).map(|s| s.to_string()) else {
            continue;
        };
        let context = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        let outcome = cursor.string(2).map(|s| s.to_string()).unwrap_or_default();
        let rationale = cursor.string(3).map(|s| s.to_string()).unwrap_or_default();
        let alts = alternatives.get(&id).cloned().unwrap_or_default();

        let text = zakhor_common::decision_text::decision_index_text(
            &context, &outcome, &rationale, &alts,
        );
        if text.is_empty() {
            continue;
        }
        docs.push(IndexableDoc { id, text });
    }
    Ok(docs)
}

/// Every document that should be searchable, observations first then decisions.
pub fn fetch_all(conn: &SparqlConnection) -> ZakhorResult<Vec<IndexableDoc>> {
    let mut docs = fetch_observations(conn)?;
    docs.extend(fetch_decisions(conn)?);
    Ok(docs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_query_targets_the_decision_class() {
        let q = decisions_query();
        assert!(q.contains("zakhor/ns/Decision"), "{q}");
        assert!(q.contains("decisionOutcome"), "{q}");
        assert!(q.contains("decisionContext"), "{q}");
        assert!(q.contains("decisionRationale"), "{q}");
    }

    #[test]
    fn alternatives_query_targets_the_alternative_predicate() {
        let q = alternatives_query();
        assert!(q.contains("zakhor/ns/Decision"), "{q}");
        assert!(q.contains("zakhor/ns/alternative"), "{q}");
    }

    /// The rebuild must not silently narrow to observations, which is what made
    /// decisions unfindable.
    #[test]
    fn observation_query_is_unchanged_and_decisions_are_covered_separately() {
        assert!(OBSERVATIONS_QUERY.contains("nie:InformationElement"));
        assert!(decisions_query().contains("zakhor/ns/Decision"));
    }
}
