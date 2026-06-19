use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod config;
mod error;
mod lexical;
mod semantic;
mod sparql;
mod server;
mod tracker_db;
mod schema;

/// Correlation ID counter shared across MCP tool invocations.
/// Each tool call gets a unique `mcp-<hex>` identifier that
/// propagates through the tracing span tree.
static CORRELATION_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Generate a unique correlation ID for an MCP request.
///
/// Format: `mcp-000001`, `mcp-000002`, … monotonically increasing
/// within a single server process.
pub fn new_correlation_id() -> String {
    use std::sync::atomic::Ordering;
    format!(
        "mcp-{:06x}",
        CORRELATION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let cfg = config::Config::load();
    let db_path = cfg.database.path.to_str().unwrap_or("./zakhor-db");
    let _conn = tracker_db::init_db(db_path);

    let service = server::MemoryHandler::new_with_config(&cfg);
    let transport = (stdin(), stdout());

    let server = service.serve(transport).await?;
    server.waiting().await?;

    Ok(())
}
