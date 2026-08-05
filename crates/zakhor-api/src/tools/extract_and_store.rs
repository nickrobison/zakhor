use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use std::sync::Arc;
use std::time::Instant;
use tracing::info_span;
use zakhor_model::pipeline::IngestionPipeline;

use crate::args::{ExtractAndStoreArgs, ExtractAndStoreResponse};
use crate::handler::{MemoryHandler, args_hash};

#[tool_router(router = tool_router_extract_and_store, vis = "pub(crate)")]
impl MemoryHandler {
    #[tool(
        description = "Extract entities and relations from text and store them in the knowledge graph"
    )]
    async fn extract_and_store(
        &self,
        Parameters(args): Parameters<ExtractAndStoreArgs>,
    ) -> Result<Json<ExtractAndStoreResponse>, String> {
        let correlation_id = crate::new_correlation_id();
        let span = info_span!(
            "mcp_tool",
            tool = "extract_and_store",
            correlation_id = %correlation_id,
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let extraction = self.ensure_extraction().await?;

        let mut pipeline = IngestionPipeline::with_sync_manager(self.sync_mgr.clone());
        let ingest_result = pipeline
            .extract_and_ingest_async(
                Arc::new(self.conn.clone()),
                &args.text,
                &extraction,
                &correlation_id,
            )
            .await
            .map_err(|e| format!("Extract and ingest failed: {e}"))?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", "success");
        span.record("duration_ms", duration_ms);

        Ok(Json(ExtractAndStoreResponse {
            observation_uri: ingest_result.observation_uri,
            entity_count: 0,
            relation_count: 0,
        }))
    }
}
