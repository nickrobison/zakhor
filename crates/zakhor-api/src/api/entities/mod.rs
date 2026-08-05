use axum::{Json, extract::Query, extract::State};
use serde::{Deserialize, Serialize};
use tracker::prelude::SparqlCursorExtManual;

use super::ApiState;
use crate::api::error::{ApiError, ApiResult};
use crate::server::EntityResult;
use crate::tools;

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

fn is_missing_schema_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("unknown class") || lower.contains("unknown property")
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct EntityListQuery {
    /// Label pattern to search for
    #[serde(default)]
    q: String,
    #[serde(default = "default_limit")]
    limit: u32,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EntityListResponse {
    pub entities: Vec<EntityResult>,
    pub count: usize,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/entities",
    params(EntityListQuery),
    responses(
        (status = OK, description = "Entity list", body = EntityListResponse),
        (status = BAD_REQUEST, description = "Invalid query", body = crate::api::error::ErrorBody)
    )
)]
pub async fn list_entities(
    State(state): State<ApiState>,
    Query(query): Query<EntityListQuery>,
) -> ApiResult<Json<EntityListResponse>> {
    let limit = clamp_limit(query.limit);
    let pattern = query.q.trim();

    let sparql = tools::build_entity_query(pattern, limit);
    let cursor = match state.connection().query(&sparql, None::<&gio::Cancellable>) {
        Ok(cursor) => cursor,
        Err(error) if is_missing_schema_error(&error.to_string()) => {
            return Ok(Json(EntityListResponse {
                entities: vec![],
                count: 0,
            }));
        }
        Err(e) => return Err(ApiError::internal(format!("SPARQL error: {e}"))),
    };

    let mut entities = Vec::new();
    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
    {
        let uri = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let label = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        entities.push(EntityResult { uri, label });
    }

    let count = entities.len();
    Ok(Json(EntityListResponse { entities, count }))
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
    fn test_is_missing_schema_error() {
        assert!(is_missing_schema_error(
            "Unknown class 'http://zakhor/ns/Entity'"
        ));
        assert!(is_missing_schema_error(
            "Unknown property 'zakhor:hasEntity'"
        ));
        assert!(!is_missing_schema_error("Cursor error"));
    }

    #[test]
    fn test_entity_list_response() {
        let resp = EntityListResponse {
            entities: vec![EntityResult {
                uri: "urn:uuid:e1".to_string(),
                label: "Entity One".to_string(),
            }],
            count: 1,
        };
        assert_eq!(resp.count, 1);
        assert_eq!(resp.entities[0].label, "Entity One");
    }
}
