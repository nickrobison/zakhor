use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use std::time::Instant;
use tracing::info_span;

use crate::args::{AdminInjectToolCallArgs, AdminInjectToolCallResponse, RebuildIndexesArgs};
use crate::handler::{MemoryHandler, args_hash};

#[tool_router(router = tool_router_admin, vis = "pub(crate)")]
impl MemoryHandler {
    #[tool(
        name = "admin_rebuild_indexes",
        description = "[Admin] Rebuild all search indexes from Tracker (ephemeral mode only)"
    )]
    async fn admin_rebuild_indexes(
        &self,
        Parameters(_args): Parameters<RebuildIndexesArgs>,
    ) -> Result<String, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "admin_rebuild_indexes",
            correlation_id = &crate::new_correlation_id(),
            args_hash = %args_hash(&_args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let start = Instant::now();

        let propagate_span = span.clone();
        let this = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _guard = propagate_span.enter();
            match &this.sync_mgr {
                Some(mgr) => mgr
                    .rebuild_all(&this.conn)
                    .map_err(|e| format!("Rebuild failed: {e}"))
                    .map(|_| "Indexes rebuilt successfully".to_string()),
                None => Err("No sync manager available (indexes disabled)".to_string()),
            }
        })
        .await
        .map_err(|e| format!("Task join error: {e}"))?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }

    #[tool(
        name = "admin_inject_tool_call",
        description = "[Admin] Inject a ToolCall node into the knowledge graph (ephemeral mode only)"
    )]
    async fn admin_inject_tool_call(
        &self,
        Parameters(args): Parameters<AdminInjectToolCallArgs>,
    ) -> Result<Json<AdminInjectToolCallResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "admin_inject_tool_call",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let result = (|| -> Result<Json<AdminInjectToolCallResponse>, String> {
            let uri = crate::tool_capture::capture_tool_call(
                &self.conn,
                &args.tool_name,
                &args.arguments,
                &args.session_id,
            )?;
            Ok(Json(AdminInjectToolCallResponse { uri }))
        })();

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }
}
