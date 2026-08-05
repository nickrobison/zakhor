use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use std::time::Instant;
use tracing::info_span;

use crate::args::{SearchHybridArgs, SearchHybridResponse, SearchResult};
use crate::handler::{args_hash, MemoryHandler};

#[tool_router(router = tool_router_search_hybrid, vis = "pub(crate)")]
impl MemoryHandler {
    #[tool(description = "Hybrid search across lexical and semantic indexes using RRF fusion")]
    async fn search_hybrid(
        &self,
        Parameters(args): Parameters<SearchHybridArgs>,
    ) -> Result<Json<SearchHybridResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "search_hybrid",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let result = match self.sync_mgr {
            Some(ref sync_mgr) => {
                let results = crate::tools::hybrid_search(sync_mgr, &args.query, args.limit as usize);
                let docs: Vec<SearchResult> = results
                    .into_iter()
                    .map(|d| SearchResult {
                        id: d.id,
                        score: d.score,
                        text: d.text,
                    })
                    .collect();
                let count = docs.len() as u64;
                Ok(Json(SearchHybridResponse {
                    results: docs,
                    count,
                    warning: None,
                }))
            }
            None => Ok(Json(SearchHybridResponse {
                results: vec![],
                count: 0,
                warning: Some("Indexes not available".to_string()),
            })),
        };

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        let result_label = if result.is_ok() { "success" } else { "error" };
        span.record("result", result_label);
        span.record("duration_ms", duration_ms);
        match &result {
            Ok(resp) => {
                let count = resp.0.count;
                let query = &args.query;
                let has_warning = resp.0.warning.is_some();
                let detail = if has_warning { " (with warning)" } else { "" };
                tracing::info!(
                    query = %query,
                    count = count,
                    "search_hybrid: {count} results for \"{query}\" in {duration_ms:.1}ms{detail}"
                );
                if let Some(w) = &resp.0.warning {
                    tracing::warn!("search_hybrid warning: {w}");
                }
            }
            Err(e) => tracing::warn!(
                error = %e,
                "search_hybrid failed: {e}"
            ),
        }
        result
    }
}
