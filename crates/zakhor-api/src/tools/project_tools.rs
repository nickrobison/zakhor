use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use std::time::Instant;
use tracing::info_span;

use crate::args::{CreateProjectArgs, CreateProjectResponse, LinkToProjectArgs};
use crate::handler::{MemoryHandler, args_hash};

#[tool_router(router = tool_router_project_tools, vis = "pub(crate)")]
impl MemoryHandler {
    #[tool(description = "Create a new project in the knowledge graph and return its URI")]
    async fn create_project(
        &self,
        Parameters(args): Parameters<CreateProjectArgs>,
    ) -> Result<Json<CreateProjectResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "create_project",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let start = Instant::now();

        let propagate_span = span.clone();
        let this = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _guard = propagate_span.enter();
            crate::project::create_project(&this.conn, &args.name, args.description.as_deref())
                .map(|project| {
                    Json(CreateProjectResponse {
                        project_uri: project.uri,
                    })
                })
                .map_err(|e| format!("Create project failed: {e}"))
        })
        .await
        .map_err(|e| format!("Task join error: {e}"))?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }

    #[tool(description = "Link an entity or decision to a project via zakhor:belongsToProject")]
    async fn link_to_project(
        &self,
        Parameters(args): Parameters<LinkToProjectArgs>,
    ) -> Result<String, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "link_to_project",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let start = Instant::now();

        let propagate_span = span.clone();
        let this = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _guard = propagate_span.enter();
            crate::project::link_to_project(&this.conn, &args.entity_uri, &args.project_uri)?;
            Ok::<String, String>(format!(
                "Linked {} to project {}",
                args.entity_uri, args.project_uri
            ))
        })
        .await
        .map_err(|e| format!("Task join error: {e}"))?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }
}
