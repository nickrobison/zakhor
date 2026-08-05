use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use std::time::Instant;
use tracing::info_span;
use tracker::prelude::SparqlCursorExtManual;

use crate::args::{EntityResult, QueryEntitiesArgs, QueryEntitiesResponse};
use crate::handler::{MemoryHandler, args_hash};

#[tool_router(router = tool_router_query_entities, vis = "pub(crate)")]
impl MemoryHandler {
    #[tool(description = "Query entities by label pattern in the knowledge graph")]
    async fn query_entities(
        &self,
        Parameters(args): Parameters<QueryEntitiesArgs>,
    ) -> Result<Json<QueryEntitiesResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "query_entities",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let result = (|| -> Result<Json<QueryEntitiesResponse>, String> {
            let sparql = crate::tools::build_entity_query(&args.pattern, args.limit);
            let cursor = self
                .conn
                .query(&sparql, None::<&gio::Cancellable>)
                .map_err(|e| format!("SPARQL query failed: {e}"))?;

            let mut entities: Vec<EntityResult> = Vec::new();
            while cursor
                .next(None::<&gio::Cancellable>)
                .map_err(|e| format!("Cursor error: {e}"))?
            {
                let uri = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
                let label = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
                entities.push(EntityResult { uri, label });
            }

            let count = entities.len() as u64;
            Ok(Json(QueryEntitiesResponse { entities, count }))
        })();

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        let result_label = if result.is_ok() { "success" } else { "error" };
        span.record("result", result_label);
        span.record("duration_ms", duration_ms);
        match &result {
            Ok(resp) => {
                let count = resp.0.count;
                let pattern = &args.pattern;
                tracing::info!(
                    pattern = %pattern,
                    count = count,
                    "query_entities: {count} results for pattern \"{pattern}\" in {duration_ms:.1}ms"
                );
            }
            Err(e) => tracing::warn!(
                error = %e,
                "query_entities failed: {e}"
            ),
        }
        result
    }
}
