use axum::routing::any_service;
use clap::Parser;
use rmcp::ServiceExt;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use std::sync::{Arc, Mutex};
use tokio::io::{stdin, stdout};
use tracing_subscriber::EnvFilter;

use zakhor_api::api::router;
use zakhor_api::server::MemoryHandler;
use zakhor_common::config::Config;
use zakhor_model::background::{self, BackgroundConfig};
use zakhor_search::IndexSyncManager;
use zakhor_storage::tracker_db;

/// Zakhor MCP server
#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = None,
    after_help = "Environment variables:\n  ZAKHOR_DB_PATH        Database path override\n  ZAKHOR_HTTP_HOST      HTTP bind host (default: 127.0.0.1)\n  ZAKHOR_HTTP_PORT      HTTP bind port (default: 3000)\n\nEphemeral mode:\n  --ephemeral           Creates a fresh Tracker DB in a temp directory (wiped on each startup)"
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

    /// Use a fresh Tracker DB in a temp directory (wiped on each startup)
    #[arg(long)]
    ephemeral: bool,
}

async fn serve_combined(
    cfg: Config,
    service: MemoryHandler,
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
    let app = router(api_state)
        .route("/", any_service(mcp_service.clone()))
        .route("/*path", any_service(mcp_service));

    axum::serve(listener, app).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(build_log_filter())
        .init();

    let cli = Cli::parse();

    let mut cfg = Config::load();
    if let Some(db_path) = cli.db_path {
        cfg.database.path = db_path;
    }

    if cli.ephemeral {
        let tmp = std::env::temp_dir().join(format!("zakhor-ephemeral-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp)?;
        cfg.database.path = tmp;
        tracing::info!(path = %cfg.database.path.display(), "Ephemeral mode — fresh Tracker DB created in temp dir");
    }

    let db_path = cfg.database.path.to_str().unwrap_or("./zakhor-db");
    let conn = tracker_db::init_db(db_path);

    let sync_mgr = if cli.rebuild_indexes {
        let mgr = IndexSyncManager::new(&cfg.database.path)?;
        mgr.rebuild_all(&conn)?;
        tracing::info!("Indexes rebuilt successfully");
        Some(Arc::new(Mutex::new(mgr)))
    } else {
        match IndexSyncManager::new(&cfg.database.path) {
            Ok(mgr) => Some(Arc::new(Mutex::new(mgr))),
            Err(e) => {
                tracing::warn!("Failed to init sync manager (indexes unavailable): {e}");
                None
            }
        }
    };

    let service = MemoryHandler::new_with_config(&cfg, sync_mgr, cli.ephemeral);

    // Start background workers (ranking refresh, stale data cleanup)
    let _bg_shutdown =
        background::start_background_workers(conn.clone(), BackgroundConfig::default());

    if cli.http {
        serve_combined(cfg, service).await?;
    } else {
        let transport = (stdin(), stdout());
        let server = service.serve(transport).await?;
        server.waiting().await?;
    }

    Ok(())
}

fn build_log_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_err| {
        // Tracing isn't initialized yet, so we can't emit a warning here.
        // Fall back to the default filter when RUST_LOG is unset or invalid.
        default_log_filter()
    })
}

fn default_log_filter() -> EnvFilter {
    EnvFilter::new("info,rmcp::service=warn")
}

#[cfg(test)]
mod tests {
    use super::default_log_filter;

    #[test]
    fn default_log_filter_suppresses_rmcp_service_info() {
        let filter = default_log_filter();
        let rendered = filter.to_string();
        assert!(
            rendered.contains("info"),
            "missing info directive: {rendered}"
        );
        assert!(
            rendered.contains("rmcp::service=warn"),
            "missing rmcp::service override: {rendered}"
        );
    }
}
