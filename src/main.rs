use axum::routing::any_service;
use clap::Parser;
use rmcp::ServiceExt;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use std::sync::{Arc, Mutex};
use tokio::io::{stdin, stdout};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod api;
mod background;
mod code_index;
mod config;
mod decision;
mod entity_resolver;
mod error;
mod ingestion;
mod lexical;
mod project;
mod provenance;
mod ranking;
mod schema;
mod semantic;
mod server;
mod sparql;
mod sync;
mod tool_capture;
mod tools;
mod tracker_db;
mod vocab;

/// Zakhor MCP server
#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = None,
    after_help = "Environment variables:\n  ZAKHOR_DB_PATH        Database path override\n  ZAKHOR_HTTP_HOST      HTTP bind host (default: 127.0.0.1)\n  ZAKHOR_HTTP_PORT      HTTP bind port (default: 3000)"
)]
struct Cli {
    /// Serve MCP over Streamable HTTP/SSE instead of stdio
    #[arg(long)]
    http: bool,

    /// Override the Tracker DB path
    #[arg(long, value_name = "PATH")]
    db_path: Option<std::path::PathBuf>,

    /// Rebuild lexical and semantic indexes before serving
    #[arg(long)]
    rebuild_indexes: bool,
}

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

async fn serve_combined(
    cfg: config::Config,
    service: server::MemoryHandler,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", cfg.http.host, cfg.http.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(addr = %addr, "Starting server");

    // Get API state before moving service into closure
    let api_state = service.api_state();

    // Create MCP HTTP service
    let mcp_service = StreamableHttpService::new(
        move || Ok(service.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_allowed_hosts([cfg.http.host.clone()]),
    );

    // Combine API router with MCP routes
    let app = api::router(api_state)
        .route("/", any_service(mcp_service.clone()))
        .route("/*path", any_service(mcp_service));

    axum::serve(listener, app).await?;
    Ok(())
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

    let cli = Cli::parse();

    let mut cfg = config::Config::load();
    if let Some(db_path) = cli.db_path {
        cfg.database.path = db_path;
    }

    let db_path = cfg.database.path.to_str().unwrap_or("./zakhor-db");
    let conn = tracker_db::init_db(db_path);

    let sync_mgr = if cli.rebuild_indexes {
        let mgr = sync::IndexSyncManager::new(&cfg.database.path)?;
        mgr.rebuild_all(&conn)?;
        tracing::info!("Indexes rebuilt successfully");
        Some(Arc::new(Mutex::new(mgr)))
    } else {
        match sync::IndexSyncManager::new(&cfg.database.path) {
            Ok(mgr) => Some(Arc::new(Mutex::new(mgr))),
            Err(e) => {
                tracing::warn!("Failed to init sync manager (indexes unavailable): {e}");
                None
            }
        }
    };

    let service = server::MemoryHandler::new_with_config(&cfg, sync_mgr);

    // Start background workers (ranking refresh, stale data cleanup)
    let _bg_shutdown =
        background::start_background_workers(conn.clone(), background::BackgroundConfig::default());

    if cli.http {
        serve_combined(cfg, service).await?;
    } else {
        let transport = (stdin(), stdout());
        let server = service.serve(transport).await?;
        server.waiting().await?;
    }

    Ok(())
}
