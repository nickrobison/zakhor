mod admin;
mod code;
mod decisions;
mod entities;
mod error;
mod graph;
mod observations;
mod projects;
mod search;

use axum::{
    Json, Router,
    extract::State,
    http::{Method, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use zakhor_model::ingestion::{EntityRef as IngEntityRef, Relation, StoreObservationArgs};
use zakhor_search::IndexSyncManager;

use crate::api::error::ErrorBody;
use crate::server::{
    EntityResult, QueryEntitiesArgs, QueryEntitiesResponse, RebuildIndexesArgs, RecordDecisionArgs,
    RecordDecisionResponse, SearchHybridArgs, SearchHybridResponse, SearchResult,
    StoreObservationResponse, TraverseGraphArgs, TraverseGraphResponse, TripleResult,
};

#[derive(Clone)]
pub struct ApiState {
    conn: tracker::SparqlConnection,
    sync_mgr: Option<Arc<Mutex<IndexSyncManager>>>,
}

impl ApiState {
    pub fn new(
        conn: tracker::SparqlConnection,
        sync_mgr: Option<Arc<Mutex<IndexSyncManager>>>,
    ) -> Self {
        Self { conn, sync_mgr }
    }

    pub fn connection(&self) -> &tracker::SparqlConnection {
        &self.conn
    }

    #[allow(dead_code)]
    pub fn sync_manager(&self) -> Option<&Arc<Mutex<IndexSyncManager>>> {
        self.sync_mgr.as_ref()
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
struct HealthResponse {
    status: &'static str,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        openapi_json,
        health,
        search::search,
        graph::traverse_graph,
        decisions::list_decisions,
        decisions::get_decision,
        decisions::get_decision_provenance,
        entities::list_entities,
        entities::get_entity,
        entities::get_entity_decisions,
        entities::get_entity_observations,
        observations::list_observations,
        observations::get_observation,
        admin::rebuild_indexes,
        admin::admin_status,
        code::get_code,
        projects::list_projects,
        projects::ranking_importance,
    ),
    components(schemas(
        HealthResponse,
        ErrorBody,
        StoreObservationArgs,
        IngEntityRef,
        Relation,
        RebuildIndexesArgs,
        QueryEntitiesArgs,
        QueryEntitiesResponse,
        EntityResult,
        TraverseGraphArgs,
        TraverseGraphResponse,
        TripleResult,
        SearchHybridArgs,
        SearchHybridResponse,
        SearchResult,
        RecordDecisionArgs,
        RecordDecisionResponse,
        StoreObservationResponse,
        decisions::DecisionSummary,
        decisions::DecisionListResponse,
        decisions::DecisionDetail,
        decisions::EvidenceItem,
        decisions::ProvenanceItem,
        decisions::ProvenanceResponse,
        entities::EntityListResponse,
        entities::EntityDetail,
        entities::EntityRef,
        entities::ObservationRef,
        entities::SourceLocation,
        entities::EntityDecisionsResponse,
        entities::DecisionRef,
        entities::EntityObservationsResponse,
        observations::ObservationSummary,
        observations::ObservationListResponse,
        observations::ObservationDetail,
        admin::RebuildResponse,
        admin::AdminStatusResponse,
        code::CodeResponse,
        code::CodeRepository,
        code::CodeFile,
        code::CodeSymbol,
        projects::ProjectItem,
        projects::ProjectListResponse,
    ))
)]
pub struct ApiDoc;

pub fn router(state: ApiState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .nest("/api/v1", routes())
        .merge(SwaggerUi::new("/api/v1/docs").url("/api/v1/openapi.json", ApiDoc::openapi()))
        .with_state(state)
        .layer(cors)
}

fn routes() -> Router<ApiState> {
    Router::new()
        .route("/health", get(health))
        .route("/search", get(search::search))
        .route("/graph/traverse", get(graph::traverse_graph))
        .route("/decisions", get(decisions::list_decisions))
        .route("/decisions/:id", get(decisions::get_decision))
        .route(
            "/decisions/:id/provenance",
            get(decisions::get_decision_provenance),
        )
        .route("/entities", get(entities::list_entities))
        .route("/entities/:id", get(entities::get_entity))
        .route(
            "/entities/:id/decisions",
            get(entities::get_entity_decisions),
        )
        .route(
            "/entities/:id/observations",
            get(entities::get_entity_observations),
        )
        .route("/observations", get(observations::list_observations))
        .route("/observations/:id", get(observations::get_observation))
        .route("/admin/rebuild-indexes", post(admin::rebuild_indexes))
        .route("/admin/status", get(admin::admin_status))
        .route("/code", get(code::get_code))
        .route("/projects", get(projects::list_projects))
        .route("/ranking/importance", get(projects::ranking_importance))
}

#[allow(dead_code)]
#[utoipa::path(
    get,
    path = "/api/v1/openapi.json",
    responses((status = OK, description = "OpenAPI 3.1 document"))
)]
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses((status = OK, description = "Tracker health check", body = HealthResponse))
)]
async fn health(State(state): State<ApiState>) -> impl IntoResponse {
    match tracker_is_available(state.connection()) {
        Ok(true) => (StatusCode::OK, Json(HealthResponse { status: "ok" })).into_response(),
        Ok(false) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "tracker_unavailable",
            }),
        )
            .into_response(),
        Err(_error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "tracker_unavailable",
            }),
        )
            .into_response(),
    }
}

fn tracker_is_available(conn: &tracker::SparqlConnection) -> Result<bool, String> {
    let cursor = conn
        .query("ASK { BIND(1 AS ?ok) }", None::<&gio::Cancellable>)
        .map_err(|e| format!("Tracker query failed: {e}"))?;

    cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| format!("Tracker cursor failed: {e}"))?;
    Ok(cursor.is_boolean(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_contains_health_schema() {
        let spec = ApiDoc::openapi();
        let value = serde_json::to_value(spec).expect("OpenAPI spec should serialize");

        assert_eq!(value["openapi"], "3.1.0");
        assert!(value["paths"]["/api/v1/health"].is_object());
        assert!(value["paths"]["/api/v1/search"].is_object());
        assert!(value["paths"]["/api/v1/graph/traverse"].is_object());
        assert!(value["components"]["schemas"]["HealthResponse"].is_object());
        assert!(value["components"]["schemas"]["ErrorBody"].is_object());
        assert!(value["components"]["schemas"]["StoreObservationArgs"].is_object());
    }
}
