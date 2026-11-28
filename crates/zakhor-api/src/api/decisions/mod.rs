use axum::{Json, extract::Query, extract::State};
use serde::{Deserialize, Serialize};
use tracker::prelude::SparqlCursorExtManual;

use super::ApiState;
use crate::api::error::{ApiError, ApiResult};
use zakhor_storage::sparql::{escape_literal, prefix_declarations};

mod detail;

pub use detail::*;

// ---------------------------------------------------------------------------
// Query parameters
// ---------------------------------------------------------------------------

fn default_limit() -> u32 {
    20
}

fn clamp_limit(limit: u32) -> u32 {
    limit.clamp(1, 100)
}

/// Decision statuses that actually exist in the model. `proposed`/`archived`
/// were previously accepted but never matchable (#96); they are now rejected
/// instead of silently returning empty result sets.
fn validate_status(status: Option<&str>) -> Result<Option<String>, ApiError> {
    match status {
        None | Some("active" | "superseded") => Ok(status.map(str::to_string)),
        Some(other) => Err(ApiError::bad_request(format!("invalid status: {other}"))),
    }
}

fn is_missing_schema_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("unknown class") || lower.contains("unknown property")
}

/// Sort keys that are backed by stored data. Tracker auto-maintains
/// `nrl:added` (creation time) and `nrl:modified` (modification counter), so
/// both orderings are real. `referenced`/`confidence` were previously accepted
/// but ignored; they are now rejected (#96).
fn validate_sort(sort: Option<&str>) -> Result<Option<String>, ApiError> {
    match sort {
        None | Some("created" | "modified") => Ok(sort.map(str::to_string)),
        Some(other) => Err(ApiError::bad_request(format!("invalid sort: {other}"))),
    }
}

/// WHERE fragment binding `?status` for filtering and display.
///
/// With a status filter the triple pattern is required so the filter compares
/// a bound value; without one the status is bound optionally for display.
fn status_pattern(status: Option<&str>) -> String {
    match status {
        Some(s) => format!(
            "  ?id zakhor:decisionStatus ?status .\n  FILTER(?status = {})\n",
            escape_literal(s)
        ),
        None => "  OPTIONAL { ?id zakhor:decisionStatus ?status . }\n".to_string(),
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
    filters.push_str(&status_pattern(status));
    filters
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct DecisionListQuery {
    /// Filter by status: active, superseded
    status: Option<String>,
    /// Sort field: created (newest first), modified (most recently modified first)
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

// ---------------------------------------------------------------------------
// SPARQL query builders
// ---------------------------------------------------------------------------

/// Build a SELECT query to list decisions with optional status filter and
/// text search over the outcome field.
///
/// `nrl:added`/`nrl:modified` are bound for ordering (and `nrl:added` is
/// returned as the creation time). They are optional because they are
/// Tracker-maintained metadata, not zakhor ontology properties.
fn build_list_query(
    q: Option<&str>,
    status: Option<&str>,
    sort: Option<&str>,
    limit: u32,
    offset: u32,
) -> String {
    let mut where_fragment = build_filter_clauses(q, status);
    where_fragment.push_str("  OPTIONAL { ?id nrl:added ?added . }\n");
    where_fragment.push_str("  OPTIONAL { ?id nrl:modified ?modified . }\n");

    // The default ordering is newest-created first; `sort` selects between
    // the same key (`created`) and Tracker's modification counter
    // (`modified`, most recently touched first).
    let order_clause = match sort {
        Some("modified") => "ORDER BY DESC(?modified)",
        _ => "ORDER BY DESC(?added)",
    };

    let prefixes = prefix_declarations();

    format!(
        "{prefixes}SELECT ?id ?outcome ?status ?added
WHERE {{
  ?id rdf:type zakhor:Decision .
{where}
}}
{order}
LIMIT {limit} OFFSET {offset}",
        prefixes = prefixes,
        where = where_fragment,
        order = order_clause,
        limit = limit,
        offset = offset,
    )
}

fn build_count_query(q: Option<&str>, status: Option<&str>) -> String {
    let filter_clause = build_filter_clauses(q, status);
    let prefixes = prefix_declarations();

    format!(
        "{prefixes}SELECT (COUNT(?id) AS ?total)
WHERE {{
  ?id rdf:type zakhor:Decision .
  OPTIONAL {{ ?id zakhor:decisionOutcome ?outcome . }}
{filter}
}}",
        prefixes = prefixes,
        filter = filter_clause,
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
            return Err(ApiError::internal(format!("SPARQL count error: {e}")));
        }
    };

    let total: usize = if count_cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
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
            return Err(ApiError::internal(format!("SPARQL data error: {e}")));
        }
    };

    let mut decisions = Vec::new();
    while data_cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
    {
        let id = data_cursor
            .string(0)
            .map(|s| s.to_string())
            .unwrap_or_default();
        let outcome = data_cursor
            .string(1)
            .map(|s| s.to_string())
            .unwrap_or_default();
        // The decision's real status; every decision created through
        // DecisionModel has one, so an empty value would signal data
        // corruption rather than a missing default.
        let status = data_cursor
            .string(2)
            .map(|s| s.to_string())
            .unwrap_or_default();
        let created = data_cursor.string(3).map(|s| s.to_string());
        decisions.push(DecisionSummary {
            id,
            title: outcome,
            status,
            created,
            // There is no per-decision modified *time*: Tracker maintains
            // nrl:modified as a modification counter, which is not a
            // displayable timestamp. Ordering by it is still honoured via
            // `sort=modified`.
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
        // Statuses that never exist in the model are rejected rather than
        // silently matching zero rows (#96).
        assert!(validate_status(Some("proposed")).is_err());
        assert!(validate_status(Some("archived")).is_err());
        assert!(validate_status(Some("invalid")).is_err());
    }

    #[test]
    fn test_validate_sort_accepts_supported_values() {
        assert_eq!(validate_sort(None).unwrap(), None);
        assert_eq!(
            validate_sort(Some("created")).unwrap(),
            Some("created".to_string())
        );
        assert_eq!(
            validate_sort(Some("modified")).unwrap(),
            Some("modified".to_string())
        );
        // Sort keys with no backing data are rejected rather than ignored (#96).
        assert!(validate_sort(Some("referenced")).is_err());
        assert!(validate_sort(Some("confidence")).is_err());
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
    fn test_status_pattern_unfiltered_binds_optionally() {
        let pattern = status_pattern(None);
        assert!(pattern.contains("OPTIONAL { ?id zakhor:decisionStatus ?status . }"));
        assert!(!pattern.contains("FILTER"));
    }

    #[test]
    fn test_status_pattern_filtered_requires_binding() {
        let pattern = status_pattern(Some("superseded"));
        assert!(pattern.contains("?id zakhor:decisionStatus ?status ."));
        assert!(pattern.contains("FILTER(?status = \"superseded\")"));
        assert!(!pattern.contains("OPTIONAL"));
    }

    #[test]
    fn test_build_list_query_default() {
        let q = build_list_query(None, None, None, 20, 0);
        assert!(q.contains("SELECT ?id ?outcome ?status ?added"));
        assert!(q.contains("LIMIT 20"));
        assert!(q.contains("OFFSET 0"));
        // Default order is newest-created first.
        assert!(q.contains("ORDER BY DESC(?added)"));
        assert!(q.contains("OPTIONAL { ?id nrl:added ?added . }"));
        assert!(q.contains("OPTIONAL { ?id nrl:modified ?modified . }"));
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
        let q = build_list_query(None, Some("superseded"), Some("created"), 10, 5);
        // The status filter matches real rows instead of FILTER(false) (#96).
        assert!(q.contains("?id zakhor:decisionStatus ?status ."));
        assert!(q.contains("FILTER(?status = \"superseded\")"));
        assert!(!q.contains("FILTER(false)"));
        assert!(q.contains("ORDER BY DESC(?added)"));
    }

    #[test]
    fn test_build_list_query_sort_modified() {
        let q = build_list_query(None, None, Some("modified"), 20, 0);
        assert!(q.contains("ORDER BY DESC(?modified)"));
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
    fn test_build_count_query_with_status_filter() {
        let q = build_count_query(None, Some("superseded"));
        assert!(q.contains("?id zakhor:decisionStatus ?status ."));
        assert!(q.contains("FILTER(?status = \"superseded\")"));
        assert!(!q.contains("FILTER(false)"));
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
}
