use super::ApiState;
use crate::api::error::{ApiError, ApiResult};
use axum::{Json, extract::State};
use serde::Serialize;
use std::sync::Arc;

use crate::sync::IndexSyncManager;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RebuildResponse {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AdminStatusResponse {
    pub rebuild_in_progress: bool,
    pub lexical_docs: u64,
    pub semantic_vectors: usize,
    pub last_rebuild_at_ms: Option<u64>,
    pub indexes_available: bool,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/admin/rebuild-indexes",
    responses(
        (status = 202, description = "Rebuild accepted", body = RebuildResponse),
        (status = 409, description = "Rebuild already in progress", body = crate::api::error::ErrorBody),
        (status = 500, description = "Sync manager not enabled", body = crate::api::error::ErrorBody)
    )
)]
pub async fn rebuild_indexes(State(state): State<ApiState>) -> ApiResult<Json<RebuildResponse>> {
    let sync_arc: Arc<std::sync::Mutex<IndexSyncManager>> = state
        .sync_manager()
        .ok_or_else(|| ApiError::internal("Index sync manager not configured"))?
        .clone();

    {
        let mgr = sync_arc
            .lock()
            .map_err(|e| ApiError::internal(format!("Sync lock: {e}")))?;
        if mgr.is_rebuild_in_progress() {
            return Err(ApiError::conflict("Rebuild already in progress"));
        }
    }

    let state_for_bg = state.clone();
    tokio::spawn(async move {
        let mgr = sync_arc
            .lock()
            .expect("sync manager lock poisoned in background task");
        let conn = state_for_bg.connection();
        match mgr.rebuild_all(conn) {
            Ok(()) => tracing::info!("Index rebuild completed"),
            Err(e) => tracing::warn!("Index rebuild error: {e}"),
        }
    });

    Ok(Json(RebuildResponse {
        status: "accepted".to_string(),
        message: "Index rebuild started".to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/status",
    responses(
        (status = OK, description = "Admin status", body = AdminStatusResponse)
    )
)]
pub async fn admin_status(State(state): State<ApiState>) -> ApiResult<Json<AdminStatusResponse>> {
    let Some(sync_arc): Option<Arc<std::sync::Mutex<IndexSyncManager>>> =
        state.sync_manager().cloned()
    else {
        return Ok(Json(AdminStatusResponse {
            rebuild_in_progress: false,
            lexical_docs: 0,
            semantic_vectors: 0,
            last_rebuild_at_ms: None,
            indexes_available: false,
        }));
    };

    let mgr = sync_arc
        .lock()
        .map_err(|e| ApiError::internal(format!("Sync lock: {e}")))?;

    let rebuild_in_progress = mgr.is_rebuild_in_progress();
    let lexical_docs = mgr.lexical.num_docs();
    let semantic_vectors = mgr.semantic.lock().map(|sem| sem.len()).unwrap_or(0);
    let last_rebuild_at_ms = mgr.last_rebuild_ms();
    let indexes_available = lexical_docs > 0 || semantic_vectors > 0;

    Ok(Json(AdminStatusResponse {
        rebuild_in_progress,
        lexical_docs,
        semantic_vectors,
        last_rebuild_at_ms,
        indexes_available,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rebuild_response() {
        let r = RebuildResponse {
            status: "accepted".into(),
            message: "started".into(),
        };
        assert_eq!(r.status, "accepted");
    }

    #[test]
    fn test_admin_status_response_defaults() {
        let r = AdminStatusResponse {
            rebuild_in_progress: false,
            lexical_docs: 0,
            semantic_vectors: 0,
            last_rebuild_at_ms: None,
            indexes_available: false,
        };
        assert!(!r.indexes_available);
        assert!(!r.rebuild_in_progress);
        assert_eq!(r.lexical_docs, 0);
    }
}
