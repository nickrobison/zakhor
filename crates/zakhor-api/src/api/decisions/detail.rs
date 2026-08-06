use axum::{Json, extract::Path, extract::State};
use serde::Serialize;
use tracker::prelude::SparqlCursorExtManual;
use zakhor_model::pipeline::EntityRef;

use super::ApiState;
use crate::api::error::{ApiError, ApiResult};
use zakhor_storage::sparql::prefix_declarations;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EvidenceItem {
    pub source: String,
    pub content: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DecisionDetail {
    pub id: String,
    pub title: String,
    pub status: String,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub context: String,
    pub outcome: String,
    pub rationale: String,
    pub alternatives: Vec<String>,
    pub evidence: Vec<EvidenceItem>,
    pub entities: Vec<EntityRef>,
    pub related_decision_ids: Vec<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProvenanceItem {
    pub step: String,
    pub label: String,
    pub source: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProvenanceResponse {
    pub chain: Vec<ProvenanceItem>,
    pub count: usize,
}

// ---------------------------------------------------------------------------
// SPARQL query builders
// ---------------------------------------------------------------------------

/// Build a SELECT query returning all (predicate, object) pairs for a
/// decision, giving us all its properties.
fn build_properties_query(decision_id: &str) -> String {
    let safe_id = decision_id.replace(['>', '<'], "");
    let prefixes = prefix_declarations();
    format!(
        "{prefixes}SELECT ?p ?o
WHERE {{
  <{id}> ?p ?o .
}}",
        prefixes = prefixes,
        id = safe_id,
    )
}

/// Build a SELECT query returning alternatives for a decision.
fn build_alternatives_query(decision_id: &str) -> String {
    let safe_id = decision_id.replace(['>', '<'], "");
    let prefixes = prefix_declarations();
    format!(
        "{prefixes}SELECT ?alt
WHERE {{
  <{id}> zakhor:decisionAlternative ?alt .
}}",
        prefixes = prefixes,
        id = safe_id,
    )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/decisions/{id}",
    params(
        ("id" = String, Path, description = "Decision ID (URI)")
    ),
    responses(
        (status = OK, description = "Decision detail", body = DecisionDetail),
        (status = NOT_FOUND, description = "Decision not found", body = crate::api::error::ErrorBody)
    )
)]
pub async fn get_decision(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> ApiResult<Json<DecisionDetail>> {
    let id = id.trim();
    if id.is_empty() {
        return Err(ApiError::bad_request("id is required"));
    }

    // Query all properties of this decision
    let sparql = build_properties_query(id);
    let cursor = state
        .connection()
        .query(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("SPARQL error: {e}")))?;

    let zakhor_prefix = "http://zakhor/ns/";
    let rdf_type = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let mut outcome = String::new();
    let mut context = String::new();
    let mut rationale = String::new();
    let mut found_decision = false;

    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
    {
        let p = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let o = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();

        if p == rdf_type && o == format!("{}Decision", zakhor_prefix) {
            found_decision = true;
        } else if p == format!("{}decisionOutcome", zakhor_prefix) {
            outcome = o;
        } else if p == format!("{}decisionContext", zakhor_prefix) {
            context = o;
        } else if p == format!("{}decisionRationale", zakhor_prefix) {
            rationale = o;
        }
    }

    if !found_decision {
        return Err(ApiError::bad_request(format!("Decision not found: {id}")));
    }

    // Fetch alternatives separately
    let alternatives = fetch_alternatives(state.connection(), id)?;

    let title = if outcome.is_empty() {
        id.to_string()
    } else {
        outcome.clone()
    };

    Ok(Json(DecisionDetail {
        id: id.to_string(),
        title,
        status: "active".to_string(),
        created: None,
        modified: None,
        context,
        outcome,
        rationale,
        alternatives,
        evidence: vec![],
        entities: vec![],
        related_decision_ids: vec![],
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/decisions/{id}/provenance",
    params(
        ("id" = String, Path, description = "Decision ID (URI)")
    ),
    responses(
        (status = OK, description = "Decision provenance chain", body = ProvenanceResponse),
        (status = BAD_REQUEST, description = "Invalid decision ID", body = crate::api::error::ErrorBody)
    )
)]
pub async fn get_decision_provenance(
    State(_state): State<ApiState>,
    Path(_id): Path<String>,
) -> ApiResult<Json<ProvenanceResponse>> {
    // Provenance is not currently modeled in the SPARQL store.
    // Return an empty chain — this will be wired when provenance data is available.
    Ok(Json(ProvenanceResponse {
        chain: vec![],
        count: 0,
    }))
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn fetch_alternatives(
    conn: &tracker::SparqlConnection,
    decision_id: &str,
) -> Result<Vec<String>, ApiError> {
    let sparql = build_alternatives_query(decision_id);
    let cursor = conn
        .query(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("SPARQL error: {e}")))?;

    let mut alternatives = Vec::new();
    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
    {
        if let Some(alt) = cursor.string(0) {
            alternatives.push(alt.to_string());
        }
    }
    Ok(alternatives)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_properties_query() {
        let q = build_properties_query("urn:uuid:abc-123");
        assert!(q.contains("<urn:uuid:abc-123>"));
        assert!(q.contains("SELECT ?p ?o"));
    }

    #[test]
    fn test_build_properties_query_escapes_angles() {
        let q = build_properties_query("<urn:uuid:abc>");
        assert!(q.contains("<urn:uuid:abc>"));
        assert!(!q.contains("<<"));
    }

    #[test]
    fn test_build_alternatives_query() {
        let q = build_alternatives_query("urn:uuid:xyz");
        assert!(q.contains("zakhor:decisionAlternative"));
        assert!(q.contains("<urn:uuid:xyz>"));
    }

    #[test]
    fn test_decision_detail_alternatives() {
        let detail = DecisionDetail {
            id: "urn:uuid:abc".to_string(),
            title: "Test".to_string(),
            status: "active".to_string(),
            created: None,
            modified: None,
            context: "ctx".to_string(),
            outcome: "out".to_string(),
            rationale: "rat".to_string(),
            alternatives: vec!["A".to_string(), "B".to_string()],
            evidence: vec![],
            entities: vec![],
            related_decision_ids: vec![],
        };
        assert_eq!(detail.alternatives.len(), 2);
        assert_eq!(detail.outcome, "out");
    }

    #[test]
    fn test_provenance_response_empty() {
        let resp = ProvenanceResponse {
            chain: vec![],
            count: 0,
        };
        assert_eq!(resp.count, 0);
        assert!(resp.chain.is_empty());
    }
}
