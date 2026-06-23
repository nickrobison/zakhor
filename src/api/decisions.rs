use super::ApiState;
use crate::api::error::ApiResult;
use axum::{Json, extract::Path, extract::Query, extract::State};
use serde::{Deserialize, Serialize};
use tracker::prelude::SparqlCursorExtManual;

// ---------------------------------------------------------------------------
// Query parameters
// ---------------------------------------------------------------------------

fn default_limit() -> u32 {
    20
}

fn clamp_limit(limit: u32) -> u32 {
    limit.clamp(1, 100)
}

fn validate_status(status: Option<&str>) -> Result<Option<String>, crate::api::error::ApiError> {
    match status {
        None | Some("active") => Ok(status.map(str::to_string)),
        Some("superseded" | "proposed" | "archived") => Ok(status.map(str::to_string)),
        Some(other) => Err(crate::api::error::ApiError::bad_request(format!(
            "invalid status: {other}"
        ))),
    }
}

fn is_missing_schema_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("unknown class") || lower.contains("unknown property")
}

fn validate_sort(sort: Option<&str>) -> Result<Option<String>, crate::api::error::ApiError> {
    match sort {
        None | Some("modified" | "created" | "referenced" | "confidence") => {
            Ok(sort.map(str::to_string))
        }
        Some(other) => Err(crate::api::error::ApiError::bad_request(format!(
            "invalid sort: {other}"
        ))),
    }
}

fn status_filter(status: Option<&str>) -> String {
    match status {
        Some("active") | None => String::new(),
        Some("superseded" | "proposed" | "archived") => "  FILTER(false)\n".to_string(),
        Some(other) => format!("  FILTER(?status = \"{other}\")\n"),
    }
}

fn build_filter_clauses(q: Option<&str>, status: Option<&str>) -> String {
    let mut filters = String::new();
    if let Some(pattern) = q.filter(|value| !value.is_empty()) {
        let safe = pattern.replace('\'', "\\'");
        filters.push_str(&format!(
            "  FILTER(CONTAINS(LCASE(?outcome), LCASE('{}')))\n",
            safe
        ));
    }
    filters.push_str(&status_filter(status));
    filters
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct DecisionListQuery {
    /// Filter by status: active, superseded, proposed, archived
    status: Option<String>,
    /// Sort field: modified, created, referenced, confidence
    sort: Option<String>,
    /// Full-text search over decision outcome
    q: Option<String>,
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default)]
    offset: u32,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DecisionSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub created: Option<String>,
    pub modified: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DecisionListResponse {
    pub decisions: Vec<DecisionSummary>,
    pub count: usize,
    pub total: usize,
}

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
    pub entities: Vec<crate::ingestion::EntityRef>,
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

/// Build a SELECT query to list decisions with optional status filter and
/// text search over the outcome field.
fn build_list_query(
    q: Option<&str>,
    status: Option<&str>,
    sort: Option<&str>,
    limit: u32,
    offset: u32,
) -> String {
    let filter_clause = build_filter_clauses(q, status);

    let order_clause = match sort {
        Some("created") => "ORDER BY ?id".to_string(),
        _ => "ORDER BY DESC(?id)".to_string(),
    };

    format!(
        "PREFIX zakhor: <http://zakhor/ns/>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
SELECT ?id ?outcome
WHERE {{
  ?id rdf:type zakhor:Decision .
  OPTIONAL {{ ?id zakhor:decisionOutcome ?outcome . }}
{filter}
}}
{order}
LIMIT {limit} OFFSET {offset}",
        filter = filter_clause,
        order = order_clause,
        limit = limit,
        offset = offset,
    )
}

fn build_count_query(q: Option<&str>, status: Option<&str>) -> String {
    let filter_clause = build_filter_clauses(q, status);

    format!(
        "PREFIX zakhor: <http://zakhor/ns/>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
SELECT (COUNT(?id) AS ?total)
WHERE {{
  ?id rdf:type zakhor:Decision .
  OPTIONAL {{ ?id zakhor:decisionOutcome ?outcome . }}
{filter}
}}",
        filter = filter_clause,
    )
}

/// Build a SELECT query returning all (predicate, object) pairs for a
/// decision, giving us all its properties.
fn build_properties_query(decision_id: &str) -> String {
    let safe_id = decision_id.replace(['>', '<'], "");
    format!(
        "PREFIX zakhor: <http://zakhor/ns/>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
SELECT ?p ?o
WHERE {{
  <{id}> ?p ?o .
}}",
        id = safe_id,
    )
}

/// Build a SELECT query returning alternatives for a decision.
fn build_alternatives_query(decision_id: &str) -> String {
    let safe_id = decision_id.replace(['>', '<'], "");
    format!(
        "PREFIX zakhor: <http://zakhor/ns/>
SELECT ?alt
WHERE {{
  <{id}> zakhor:decisionAlternative ?alt .
}}",
        id = safe_id,
    )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/decisions",
    params(DecisionListQuery),
    responses(
        (status = OK, description = "Decision list", body = DecisionListResponse),
        (status = BAD_REQUEST, description = "Invalid query params", body = crate::api::error::ErrorBody)
    )
)]
pub async fn list_decisions(
    State(state): State<ApiState>,
    Query(query): Query<DecisionListQuery>,
) -> ApiResult<Json<DecisionListResponse>> {
    let limit = clamp_limit(query.limit);
    let q = query.q.as_deref();
    let status = validate_status(query.status.as_deref())?;
    let sort = validate_sort(query.sort.as_deref())?;

    // Total count query
    let count_sparql = build_count_query(q, status.as_deref());
    let count_cursor = match state
        .connection()
        .query(&count_sparql, None::<&gio::Cancellable>)
    {
        Ok(cursor) => cursor,
        Err(error) if is_missing_schema_error(&error.to_string()) => {
            return Ok(Json(DecisionListResponse {
                decisions: vec![],
                count: 0,
                total: 0,
            }));
        }
        Err(e) => {
            return Err(crate::api::error::ApiError::internal(format!(
                "SPARQL count error: {e}"
            )));
        }
    };

    let total: usize = if count_cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| crate::api::error::ApiError::internal(format!("Cursor error: {e}")))?
    {
        count_cursor.integer(0).max(0) as usize
    } else {
        0
    };

    // Data query
    let data_sparql = build_list_query(q, status.as_deref(), sort.as_deref(), limit, query.offset);
    let data_cursor = match state
        .connection()
        .query(&data_sparql, None::<&gio::Cancellable>)
    {
        Ok(cursor) => cursor,
        Err(error) if is_missing_schema_error(&error.to_string()) => {
            return Ok(Json(DecisionListResponse {
                decisions: vec![],
                count: 0,
                total,
            }));
        }
        Err(e) => {
            return Err(crate::api::error::ApiError::internal(format!(
                "SPARQL data error: {e}"
            )));
        }
    };

    let mut decisions = Vec::new();
    while data_cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| crate::api::error::ApiError::internal(format!("Cursor error: {e}")))?
    {
        let id = data_cursor
            .string(0)
            .map(|s| s.to_string())
            .unwrap_or_default();
        let outcome = data_cursor
            .string(1)
            .map(|s| s.to_string())
            .unwrap_or_default();
        decisions.push(DecisionSummary {
            id,
            title: outcome.clone(),
            status: "active".to_string(),
            created: None,
            modified: None,
        });
    }

    let count = decisions.len();
    Ok(Json(DecisionListResponse {
        decisions,
        count,
        total,
    }))
}

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
        return Err(crate::api::error::ApiError::bad_request("id is required"));
    }

    // Query all properties of this decision
    let sparql = build_properties_query(id);
    let cursor = state
        .connection()
        .query(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| crate::api::error::ApiError::internal(format!("SPARQL error: {e}")))?;

    let zakhor_prefix = "http://zakhor/ns/";
    let rdf_type = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let mut outcome = String::new();
    let mut context = String::new();
    let mut rationale = String::new();
    let mut found_decision = false;

    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| crate::api::error::ApiError::internal(format!("Cursor error: {e}")))?
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
        return Err(crate::api::error::ApiError::bad_request(format!(
            "Decision not found: {id}"
        )));
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
) -> Result<Vec<String>, crate::api::error::ApiError> {
    let sparql = build_alternatives_query(decision_id);
    let cursor = conn
        .query(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| crate::api::error::ApiError::internal(format!("SPARQL error: {e}")))?;

    let mut alternatives = Vec::new();
    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| crate::api::error::ApiError::internal(format!("Cursor error: {e}")))?
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
    fn test_default_limit() {
        assert_eq!(default_limit(), 20);
    }

    #[test]
    fn test_clamp_limit() {
        assert_eq!(clamp_limit(0), 1);
        assert_eq!(clamp_limit(50), 50);
        assert_eq!(clamp_limit(500), 100);
    }

    #[test]
    fn test_validate_status_accepts_supported_values() {
        assert_eq!(validate_status(None).unwrap(), None);
        assert_eq!(
            validate_status(Some("active")).unwrap(),
            Some("active".to_string())
        );
        assert_eq!(
            validate_status(Some("superseded")).unwrap(),
            Some("superseded".to_string())
        );
        assert!(validate_status(Some("invalid")).is_err());
    }

    #[test]
    fn test_validate_sort_accepts_supported_values() {
        assert_eq!(validate_sort(None).unwrap(), None);
        assert_eq!(
            validate_sort(Some("modified")).unwrap(),
            Some("modified".to_string())
        );
        assert_eq!(
            validate_sort(Some("created")).unwrap(),
            Some("created".to_string())
        );
        assert_eq!(
            validate_sort(Some("referenced")).unwrap(),
            Some("referenced".to_string())
        );
        assert_eq!(
            validate_sort(Some("confidence")).unwrap(),
            Some("confidence".to_string())
        );
        assert!(validate_sort(Some("invalid")).is_err());
    }

    #[test]
    fn test_is_missing_schema_error() {
        assert!(is_missing_schema_error(
            "Unknown class 'http://zakhor/ns/Decision'"
        ));
        assert!(is_missing_schema_error(
            "Unknown property 'zakhor:decisionOutcome'"
        ));
        assert!(!is_missing_schema_error("Cursor error"));
    }

    #[test]
    fn test_build_list_query_default() {
        let q = build_list_query(None, None, None, 20, 0);
        assert!(q.contains("SELECT ?id ?outcome"));
        assert!(q.contains("LIMIT 20"));
        assert!(q.contains("OFFSET 0"));
        assert!(q.contains("ORDER BY DESC(?id)"));
        assert!(!q.contains("FILTER(CONTAINS"));
        assert!(!q.contains("FILTER(false)"));
    }

    #[test]
    fn test_build_list_query_with_q() {
        let q = build_list_query(Some("test"), None, None, 10, 5);
        assert!(q.contains("CONTAINS(LCASE(?outcome), LCASE('test'))"));
        assert!(q.contains("LIMIT 10"));
        assert!(q.contains("OFFSET 5"));
    }

    #[test]
    fn test_build_list_query_with_status() {
        let q = build_list_query(None, Some("archived"), Some("created"), 10, 5);
        assert!(q.contains("FILTER(false)"));
        assert!(q.contains("ORDER BY ?id"));
    }

    #[test]
    fn test_build_list_query_escapes_quotes() {
        let q = build_list_query(Some("it's"), None, None, 20, 0);
        assert!(q.contains("it\\'s"));
    }

    #[test]
    fn test_build_count_query() {
        let q = build_count_query(None, None);
        assert!(q.contains("COUNT(?id)"));
        assert!(q.contains("AS ?total"));
    }

    #[test]
    fn test_build_count_query_with_q() {
        let q = build_count_query(Some("search"), None);
        assert!(q.contains("CONTAINS(LCASE(?outcome), LCASE('search'))"));
    }

    #[test]
    fn test_build_count_query_with_inactive_status() {
        let q = build_count_query(None, Some("superseded"));
        assert!(q.contains("FILTER(false)"));
    }

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
    fn test_decision_summary_schema() {
        let summary = DecisionSummary {
            id: "urn:uuid:abc".to_string(),
            title: "Test decision".to_string(),
            status: "active".to_string(),
            created: None,
            modified: None,
        };
        assert_eq!(summary.id, "urn:uuid:abc");
        assert_eq!(summary.status, "active");
    }

    #[test]
    fn test_decision_list_response_defaults() {
        let resp = DecisionListResponse {
            decisions: vec![],
            count: 0,
            total: 0,
        };
        assert_eq!(resp.decisions.len(), 0);
        assert_eq!(resp.count, 0);
        assert_eq!(resp.total, 0);
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
