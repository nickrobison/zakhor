use axum::{Json, extract::Query, extract::State};
use serde::Deserialize;
use tracker::prelude::SparqlCursorExtManual;

use super::ApiState;
use crate::api::error::{ApiError, ApiResult};
use crate::server::{TraverseGraphResponse, TripleResult};
use crate::tools;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct GraphQuery {
    start_id: String,
    #[serde(default = "default_depth")]
    depth: u32,
    edge_types: Option<String>,
}

fn default_depth() -> u32 {
    1
}

fn clamp_depth(depth: u32) -> u32 {
    depth.clamp(1, 3)
}

fn split_edge_types(edge_types: Option<String>) -> Vec<String> {
    edge_types
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[utoipa::path(
    get,
    path = "/api/v1/graph/traverse",
    params(GraphQuery),
    responses(
        (status = OK, description = "Graph traversal triples", body = TraverseGraphResponse),
        (status = BAD_REQUEST, description = "Invalid graph query", body = crate::api::error::ErrorBody)
    )
)]
pub async fn traverse_graph(
    State(state): State<ApiState>,
    Query(query): Query<GraphQuery>,
) -> ApiResult<Json<TraverseGraphResponse>> {
    let start_id = query.start_id.trim();
    if start_id.is_empty() {
        return Err(ApiError::bad_request(
            "start_id is required",
        ));
    }

    let depth = clamp_depth(query.depth);
    let edge_types = split_edge_types(query.edge_types);
    let sparql = tools::build_traverse_query(start_id, depth, &edge_types);
    let cursor = match state.connection().query(&sparql, None::<&gio::Cancellable>) {
        Ok(cursor) => cursor,
        Err(error) => {
            return Ok(Json(TraverseGraphResponse {
                triples: vec![],
                count: 0,
                warning: Some(format!("Query issue: {error}")),
            }));
        }
    };

    let mut triples = Vec::new();
    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
    {
        let subject = cursor
            .string(0)
            .map(|value| value.to_string())
            .unwrap_or_default();
        let predicate = cursor
            .string(1)
            .map(|value| value.to_string())
            .unwrap_or_default();
        let object = cursor
            .string(2)
            .map(|value| value.to_string())
            .unwrap_or_default();
        triples.push(TripleResult {
            subject,
            predicate,
            object,
        });
    }

    let count = triples.len() as u64;
    Ok(Json(TraverseGraphResponse {
        triples,
        count,
        warning: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::TraverseGraphArgs;

    #[test]
    fn test_default_depth() {
        assert_eq!(default_depth(), 1);
    }

    #[test]
    fn test_clamp_depth() {
        assert_eq!(clamp_depth(0), 1);
        assert_eq!(clamp_depth(2), 2);
        assert_eq!(clamp_depth(10), 3);
    }

    #[test]
    fn test_split_edge_types() {
        assert_eq!(split_edge_types(None), Vec::<String>::new());
        assert_eq!(
            split_edge_types(Some("a, b,,c".to_string())),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_graph_query_schema() {
        let args = TraverseGraphArgs {
            start_id: "urn:test".to_string(),
            depth: 2,
            edge_types: vec!["related".to_string()],
        };
        assert_eq!(args.depth, 2);
    }
}
