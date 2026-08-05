use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use std::sync::Arc;
use std::time::Instant;
use tracing::info_span;
use zakhor_model::pipeline::{IngestionPipeline, StoreObservationArgs};

use crate::args::StoreObservationResponse;
use crate::handler::{MemoryHandler, args_hash};

#[tool_router(router = tool_router_store_observation, vis = "pub(crate)")]
impl MemoryHandler {
    #[tool(
        description = "Store an observation about entities and relations in the knowledge graph"
    )]
    async fn store_observation(
        &self,
        Parameters(args): Parameters<StoreObservationArgs>,
    ) -> Result<Json<StoreObservationResponse>, String> {
        let correlation_id = crate::new_correlation_id();
        let span = info_span!(
            "mcp_tool",
            tool = "store_observation",
            correlation_id = %correlation_id,
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let mut pipeline = IngestionPipeline::with_sync_manager(self.sync_mgr.clone());
        let ingest_result = pipeline
            .ingest_async(Arc::new(self.conn.clone()), args, &correlation_id)
            .await
            .map_err(|e| format!("Ingest failed: {e}"))?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", "success");
        span.record("duration_ms", duration_ms);

        Ok(Json(StoreObservationResponse {
            observation_uri: ingest_result.observation_uri,
            triple_count: ingest_result.triple_count as u64,
        }))
    }
}
