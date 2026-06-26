use axum::{Json, extract::Query, extract::State};
use serde::{Deserialize, Serialize};

use super::ApiState;
use crate::api::error::{ApiError, ApiResult};
use crate::server::{SearchHybridResponse, SearchResult};
use crate::tools;

fn default_limit() -> u32 {
    20
}

fn clamp_limit(limit: u32) -> u32 {
    limit.clamp(1, 100)
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    #[default]
    Hybrid,
    Lexical,
    Semantic,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct SearchQuery {
    #[serde(default)]
    q: String,
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default)]
    #[param(value_type = String, example = "hybrid")]
    mode: SearchMode,
}

#[utoipa::path(
    get,
    path = "/api/v1/search",
    params(
        ("q" = String, Query, description = "Search query"),
        ("limit" = u32, Query, description = "Maximum number of results"),
        ("mode" = String, Query, description = "Search mode", example = json!("hybrid"))
    ),
    responses(
        (status = OK, description = "Hybrid search results", body = SearchHybridResponse),
        (status = BAD_REQUEST, description = "Missing search query", body = crate::api::error::ErrorBody)
    )
)]
pub async fn search(
    State(state): State<ApiState>,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<SearchHybridResponse>> {
    let query_text = query.q.trim();
    if query_text.is_empty() {
        return Err(ApiError::bad_request("q is required"));
    }

    let limit = clamp_limit(query.limit) as usize;
    let Some(sync_mgr) = state.sync_manager() else {
        return Ok(Json(SearchHybridResponse {
            results: vec![],
            count: 0,
            warning: Some("Indexes not available".to_string()),
        }));
    };

    let mgr = sync_mgr
        .lock()
        .map_err(|_| ApiError::conflict("Sync manager lock poisoned"))?;
    let docs = match query.mode {
        SearchMode::Hybrid => tools::hybrid_search(&mgr.lexical, &mgr.semantic, query_text, limit),
        SearchMode::Lexical => mgr
            .lexical
            .search(query_text, limit)
            .map_err(|e| ApiError::internal(format!("Lexical search failed: {e}")))?,
        SearchMode::Semantic => mgr
            .semantic
            .lock()
            .map_err(|_| ApiError::conflict("Semantic index lock poisoned"))?
            .search(query_text, limit),
    };
    let results = docs
        .into_iter()
        .map(|doc| SearchResult {
            id: doc.id,
            score: doc.score,
        })
        .collect::<Vec<_>>();
    let count = results.len() as u64;

    Ok(Json(SearchHybridResponse {
        results,
        count,
        warning: None,
    }))
}

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
}
