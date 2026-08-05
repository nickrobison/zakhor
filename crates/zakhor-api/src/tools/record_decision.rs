use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use std::time::Instant;
use tracing::info_span;
use zakhor_model::decision::{CreateDecisionArgs, DecisionModel};

use crate::args::{RecordDecisionArgs, RecordDecisionResponse};
use crate::handler::{args_hash, MemoryHandler};

#[tool_router(router = tool_router_record_decision, vis = "pub(crate)")]
impl MemoryHandler {
    #[tool(
        description = "Record a decision with context, alternatives, and rationale in the knowledge graph"
    )]
    async fn record_decision(
        &self,
        Parameters(args): Parameters<RecordDecisionArgs>,
    ) -> Result<Json<RecordDecisionResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "record_decision",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let result = (|| -> Result<Json<RecordDecisionResponse>, String> {
            let decision_args = CreateDecisionArgs {
                context: args.context,
                outcome: args.decision,
                alternatives: args.alternatives,
                rationale: args.rationale,
                affects: vec![],
                derived_from: vec![],
                supersedes: None,
                conflicts_with: vec![],
                depends_on: vec![],
                project_uri: None,
            };
            let create_result = DecisionModel::create(&self.conn, decision_args)?;

            Ok(Json(RecordDecisionResponse {
                decision_uri: create_result.decision_uri.as_str().to_string(),
            }))
        })();

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }
}
