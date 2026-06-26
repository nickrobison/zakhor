use axum::{Json, extract::Path, extract::Query, extract::State};
use serde::{Deserialize, Serialize};
use tracker::prelude::SparqlCursorExtManual;

use super::ApiState;
use crate::api::error::{ApiError, ApiResult};

// ---------------------------------------------------------------------------
// Query parameters
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ObservationQuery {
    #[serde(default)]
    entity_id: Option<String>,
    #[serde(default = "default_offset")]
    offset: u32,
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_offset() -> u32 {
    0
}

fn default_limit() -> u32 {
    20
}

fn clamp_limit(limit: u32) -> u32 {
    limit.clamp(1, 100)
}

fn sanitize_id(id: &str) -> String {
    id.replace(['>', '<'], "")
}

fn observation_entity_filter(entity_id: Option<&str>) -> String {
    match entity_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) => format!(
            "  FILTER EXISTS {{ <{}> zakhor:hasEntity ?entity . }}",
            sanitize_id(id)
        ),
        None => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ObservationSummary {
    pub id: String,
    pub text: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ObservationListResponse {
    pub observations: Vec<ObservationSummary>,
    pub count: usize,
    pub total: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ObservationDetail {
    pub id: String,
    pub content: String,
    pub created_at: Option<String>,
    pub entity_refs: Vec<String>,
}

// ---------------------------------------------------------------------------
// SPARQL query builders
// ---------------------------------------------------------------------------

/// SELECT query returning a paginated list of observations.
fn build_all_observations_query(offset: u32, limit: u32, entity_id: Option<&str>) -> String {
    let filter = observation_entity_filter(entity_id);
    let filter_block = if filter.is_empty() {
        String::new()
    } else {
        format!("{filter}\n")
    };

    format!(
        "PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
SELECT DISTINCT ?obs ?identifier ?text ?created
WHERE {{
  ?obs rdf:type nie:InformationElement ;
       nie:identifier ?identifier ;
       nie:plainTextContent ?text .
  OPTIONAL {{ ?obs nie:contentCreated ?created . }}
{filter_block}}}
ORDER BY DESC(?obs)
LIMIT {limit}
OFFSET {offset}"
    )
}

/// SELECT COUNT query for the total number of observations.
fn build_observations_count_query(entity_id: Option<&str>) -> String {
    let filter = observation_entity_filter(entity_id);
    let filter_block = if filter.is_empty() {
        String::new()
    } else {
        format!("{filter}\n")
    };

    format!(
        "PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
SELECT (COUNT(?obs) AS ?count)
WHERE {{
  ?obs rdf:type nie:InformationElement .
{filter_block}}}"
    )
}

/// SELECT query returning entities referenced by an observation.
fn build_observation_entities_query(obs_id: &str) -> String {
    let safe_id = sanitize_id(obs_id);
    format!(
        "PREFIX zakhor: <https://zakhor.example/>
SELECT DISTINCT ?entity
WHERE {{
  <{id}> zakhor:hasEntity ?entity .
}}",
        id = safe_id,
    )
}

/// SELECT query returning a single observation's detail fields.
fn build_observation_detail_query(obs_id: &str) -> String {
    let safe_id = sanitize_id(obs_id);
    format!(
        "PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
SELECT ?identifier ?text ?created
WHERE {{
  <{id}> rdf:type nie:InformationElement ;
         nie:identifier ?identifier ;
         nie:plainTextContent ?text .
  OPTIONAL {{ <{id}> nie:contentCreated ?created . }}
}}",
        id = safe_id,
    )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/observations",
    params(ObservationQuery),
    responses(
        (status = OK, description = "Observation list", body = ObservationListResponse),
        (status = BAD_REQUEST, description = "Invalid query", body = crate::api::error::ErrorBody)
    )
)]
pub async fn list_observations(
    State(state): State<ApiState>,
    Query(query): Query<ObservationQuery>,
) -> ApiResult<Json<ObservationListResponse>> {
    let limit = clamp_limit(query.limit);
    let offset = query.offset;
    let entity_id = query
        .entity_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty());
    if entity_id.is_some() {
        return Ok(Json(ObservationListResponse {
            observations: vec![],
            count: 0,
            total: 0,
        }));
    }

    // Total count
    let total = {
        let cursor = state
            .connection()
            .query(
                &build_observations_count_query(entity_id),
                None::<&gio::Cancellable>,
            )
            .map_err(|e| ApiError::internal(format!("SPARQL count error: {e}")))?;
        let mut total: u64 = 0;
        if cursor
            .next(None::<&gio::Cancellable>)
            .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
        {
            total = cursor.integer(0).max(0) as u64;
        }
        total
    };

    // Paginated results
    let sparql = build_all_observations_query(offset, limit, entity_id);
    let cursor = state
        .connection()
        .query(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("SPARQL error: {e}")))?;

    let mut observations = Vec::new();
    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
    {
        let id = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let text = cursor.string(2).map(|s| s.to_string()).unwrap_or_default();
        let created_at = cursor.string(3).map(|s| s.to_string());
        observations.push(ObservationSummary {
            id,
            text,
            created_at,
        });
    }

    let count = observations.len();
    Ok(Json(ObservationListResponse {
        observations,
        count,
        total,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/observations/{id}",
    params(
        ("id" = String, Path, description = "Observation ID (SPARQL subject URI)")
    ),
    responses(
        (status = OK, description = "Observation detail", body = ObservationDetail),
        (status = NOT_FOUND, description = "Observation not found", body = crate::api::error::ErrorBody)
    )
)]
pub async fn get_observation(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ObservationDetail>> {
    let id = id.trim();
    if id.is_empty() {
        return Err(ApiError::bad_request("id is required"));
    }

    // Fetch detail fields
    let cursor = state
        .connection()
        .query(
            &build_observation_detail_query(id),
            None::<&gio::Cancellable>,
        )
        .map_err(|e| ApiError::internal(format!("SPARQL error: {e}")))?;

    let mut content = String::new();
    let mut created_at: Option<String> = None;
    let mut found = false;

    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
    {
        found = true;
        content = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        created_at = cursor.string(2).map(|s| s.to_string());
    }

    if !found {
        return Err(ApiError::not_found(format!("Observation not found: {id}")));
    }

    // Fetch entity references
    let entity_refs = {
        let cursor = state
            .connection()
            .query(
                &build_observation_entities_query(id),
                None::<&gio::Cancellable>,
            )
            .map_err(|e| ApiError::internal(format!("SPARQL error: {e}")))?;
        let mut refs = Vec::new();
        while cursor
            .next(None::<&gio::Cancellable>)
            .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
        {
            if let Some(entity) = cursor.string(0).map(|s| s.to_string()) {
                refs.push(entity);
            }
        }
        refs
    };

    Ok(Json(ObservationDetail {
        id: id.to_string(),
        content,
        created_at,
        entity_refs,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_offset() {
        assert_eq!(default_offset(), 0);
    }

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
    fn test_build_all_observations_query() {
        let q = build_all_observations_query(10, 20, None);
        assert!(q.contains("nie:InformationElement"));
        assert!(q.contains("nie:plainTextContent"));
        assert!(q.contains("LIMIT 20"));
        assert!(q.contains("OFFSET 10"));
        assert!(q.contains("nie:contentCreated"));
    }

    #[test]
    fn test_build_all_observations_query_with_entity_filter() {
        let q = build_all_observations_query(0, 10, Some("http://example.org/entity"));
        assert!(q.contains("<http://example.org/entity> zakhor:hasEntity ?entity"));
    }

    #[test]
    fn test_build_observations_count_query() {
        let q = build_observations_count_query(None);
        assert!(q.contains("COUNT(?obs)"));
        assert!(q.contains("nie:InformationElement"));
    }

    #[test]
    fn test_build_observations_count_query_with_entity_filter() {
        let q = build_observations_count_query(Some("urn:uuid:entity"));
        assert!(q.contains("<urn:uuid:entity> zakhor:hasEntity ?entity"));
    }

    #[test]
    fn test_build_observation_entities_query() {
        let q = build_observation_entities_query("urn:uuid:obs-1");
        assert!(q.contains("zakhor:hasEntity"));
        assert!(q.contains("<urn:uuid:obs-1>"));
    }

    #[test]
    fn test_build_observation_detail_query() {
        let q = build_observation_detail_query("urn:uuid:obs-1");
        assert!(q.contains("nie:plainTextContent"));
        assert!(q.contains("nie:contentCreated"));
        assert!(q.contains("<urn:uuid:obs-1>"));
    }

    #[test]
    fn test_observation_summary() {
        let s = ObservationSummary {
            id: "obs-1".into(),
            text: "hello".into(),
            created_at: Some("2024-01-01".into()),
        };
        assert_eq!(s.id, "obs-1");
        assert_eq!(s.created_at.as_deref(), Some("2024-01-01"));
    }

    #[test]
    fn test_observation_list_response() {
        let r = ObservationListResponse {
            observations: vec![],
            count: 0,
            total: 42,
        };
        assert_eq!(r.total, 42);
    }

    #[test]
    fn test_observation_detail() {
        let d = ObservationDetail {
            id: "obs-1".into(),
            content: "detail".into(),
            created_at: None,
            entity_refs: vec!["urn:entity:1".into()],
        };
        assert_eq!(d.entity_refs.len(), 1);
    }
}
