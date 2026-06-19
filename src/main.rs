use axum::{Router, routing::any_service};
use rmcp::ServiceExt;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use std::sync::{Arc, Mutex};
use tokio::io::{stdin, stdout};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod config;
mod error;
mod ingestion;
mod lexical;
mod provenance;
mod schema;
mod semantic;
mod server;
mod sparql;
mod sync;
mod tools;
mod tracker_db;

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

async fn serve_http(
    cfg: &config::Config,
    service: server::MemoryHandler,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", cfg.http.host, cfg.http.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(addr = %addr, "Starting HTTP transport mode");

    let http_service = StreamableHttpService::new(
        move || Ok(service.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_allowed_hosts([cfg.http.host.clone()]),
    );

    let app = Router::new()
        .route("/", any_service(http_service.clone()))
        .route("/*path", any_service(http_service));

    axum::serve(listener, app).await?;
    Ok(())
}

fn print_help() {
    println!(
        "Zakhor MCP server\n\n\
Usage:\n  zakhor [OPTIONS]\n\n\
Options:\n  --http              Serve MCP over Streamable HTTP/SSE instead of stdio\n  --db-path <PATH>    Override the Tracker DB path\n  --rebuild-indexes   Rebuild lexical and semantic indexes before serving\n  -h, --help          Print this help text\n\n\
Environment:\n  ZAKHOR_DB_PATH        Database path override\n  ZAKHOR_HTTP_HOST      HTTP bind host (default: 127.0.0.1)\n  ZAKHOR_HTTP_PORT      HTTP bind port (default: 3000)"
    );
}

fn apply_cli_overrides(cfg: &mut config::Config) {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--db-path" => {
                if i + 1 < args.len() {
                    cfg.database.path = std::path::PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            other if other.starts_with("--db-path=") => {
                cfg.database.path = std::path::PathBuf::from(&other["--db-path=".len()..]);
            }
            _ => {}
        }
        i += 1;
    }
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

    let mut cfg = config::Config::load();
    apply_cli_overrides(&mut cfg);
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    let db_path = cfg.database.path.to_str().unwrap_or("./zakhor-db");
    let conn = tracker_db::init_db(db_path);

    let rebuild = args.iter().any(|a| a == "--rebuild-indexes");
    let http_mode = args.iter().any(|a| a == "--http");

    let sync_mgr = if rebuild {
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

    if http_mode {
        serve_http(&cfg, service).await?;
    } else {
        let transport = (stdin(), stdout());
        let server = service.serve(transport).await?;
        server.waiting().await?;
    }

    Ok(())
}
