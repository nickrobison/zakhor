//! Project API endpoints

use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;
use zakhor_model::ranking::ScoredEntity;

use super::ApiState;
use crate::api::error::{ApiError, ApiResult};

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectItem {
    pub uri: String,
    pub label: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectItem>,
    pub count: usize,
}

#[utoipa::path(
    get,
    path = "/api/v1/projects",
    responses(
        (status = OK, description = "List all projects", body = ProjectListResponse),
    )
)]
pub async fn list_projects(State(state): State<ApiState>) -> ApiResult<Json<ProjectListResponse>> {
    let projects = crate::project::list_projects(state.connection()).map_err(ApiError::internal)?;
    let items: Vec<ProjectItem> = projects
        .into_iter()
        .map(|p| ProjectItem {
            uri: p.uri,
            label: p.name,
        })
        .collect();
    let count = items.len();
    Ok(Json(ProjectListResponse {
        projects: items,
        count,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/ranking/importance",
    responses(
        (status = OK, description = "Graph importance ranking"),
    )
)]
pub async fn ranking_importance(
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<ScoredEntity>>> {
    let result = zakhor_model::ranking::compute_importance(state.connection())
        .map_err(ApiError::internal)?;
    Ok(Json(result))
}
