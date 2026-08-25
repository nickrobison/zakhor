use axum::routing::any_service;
use clap::Parser;
use clap::ValueEnum;
use rmcp::ServiceExt;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use std::sync::Arc;
use tokio::io::{stdin, stdout};
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;

use zakhor_api::api::router;
use zakhor_api::server::MemoryHandler;
use zakhor_common::config::Config;
use zakhor_model::background::{self, BackgroundConfig};
use zakhor_search::IndexSyncManager;
use zakhor_storage::tracker_db;

// ---------------------------------------------------------------------------
// Logging CLI
// ---------------------------------------------------------------------------

/// Log verbosity level.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

/// Log output format.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum LogFormat {
    /// Structured JSON (good for production log pipelines)
    Json,
    /// Human-readable pretty-printed output
    Pretty,
}

// ---------------------------------------------------------------------------
// Application CLI
// ---------------------------------------------------------------------------

/// Zakhor MCP server
#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = None,
    after_help = "Environment variables:\n  ZAKHOR_DB_PATH        Database path override\n  ZAKHOR_HTTP_HOST      HTTP bind host (default: 127.0.0.1)\n  ZAKHOR_HTTP_PORT      HTTP bind port (default: 3000)\n\nConfig file:\n  -c, --config PATH     Path to the TOML configuration file (default: zakhor.toml)\n\nEphemeral mode:\n  --ephemeral           Creates a fresh Tracker DB in a temp directory (wiped on each startup)\n\nLogging:\n  RUST_LOG              Overrides --log-level for crate-level control (e.g. \"debug,zakhor_api=trace\")"
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

    /// Path to the TOML configuration file
    #[arg(short = 'c', long, value_name = "PATH", default_value = "zakhor.toml")]
    config: std::path::PathBuf,

    /// Log level (overridden by RUST_LOG if set)
    #[arg(long, value_enum, default_value_t = LogLevel::Debug)]
    log_level: LogLevel,

    /// Increase verbosity (repeat for more: -v = debug, -vv = trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Log output format
    #[arg(long, value_enum, default_value_t = LogFormat::Pretty)]
    log_format: LogFormat,
}

#[tracing::instrument(skip(cfg, service))]
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
        StreamableHttpServerConfig::default()
            .with_allowed_hosts(["localhost".to_string(), cfg.http.host.clone()]),
    );

    // SPA static file service: serve ui/dist with index.html fallback for client-side routing
    if !std::path::Path::new("ui/dist/index.html").exists() {
        tracing::warn!(
            "ui/dist/index.html not found; UI will not be served. Build the frontend first (cd ui && pnpm run build)."
        );
    }
    let spa_service =
        ServeDir::new("ui/dist").not_found_service(ServeFile::new("ui/dist/index.html"));

    // Combine API router with MCP routes and SPA fallback
    let app = router(api_state)
        .route("/mcp", any_service(mcp_service.clone()))
        .route("/mcp/*path", any_service(mcp_service))
        .fallback_service(spa_service);

    axum::serve(listener, app).await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let log_filter = build_log_filter(cli.log_level, cli.verbose);
    let subscriber = tracing_subscriber::fmt().with_env_filter(log_filter);
    match cli.log_format {
        LogFormat::Json => subscriber.json().init(),
        LogFormat::Pretty => subscriber.pretty().init(),
    }

    // Initialize ONNX Runtime globally with the CPU execution provider.
    // This configures the process-wide `OrtEnv` used by all sessions (fastembed
    // for embeddings, orp/gline-rs for GLiNER/RELEX). Without at least one EP
    // registered, `Session::builder().commit_from_file(...)` hangs.
    if !ort::init()
        .with_execution_providers([ort::ep::CPU::default().build()])
        .commit()
    {
        tracing::warn!(
            "ORT already initialized (benign — global env already set by another crate)"
        );
    }

    let mut cfg = Config::load_from(&cli.config);
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

    let embedding_enabled = cfg.embedding.enabled;
    let sync_mgr = if cli.rebuild_indexes {
        let mgr = IndexSyncManager::new(&cfg.database.path, embedding_enabled)?;
        mgr.rebuild_all(&conn)?;
        tracing::info!("Indexes rebuilt successfully");
        Some(Arc::new(mgr))
    } else {
        match IndexSyncManager::new(&cfg.database.path, embedding_enabled) {
            Ok(mgr) => Some(Arc::new(mgr)),
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

fn build_log_filter(cli_level: LogLevel, verbose: u8) -> EnvFilter {
    // RUST_LOG env var takes full control when set
    if let Ok(filter) = EnvFilter::try_from_default_env() {
        return filter;
    }

    let level = match verbose {
        0 => cli_level,
        1 => LogLevel::Debug,
        _ => LogLevel::Trace,
    };

    EnvFilter::new(format!(
        "{level},rmcp::service=warn,h2=warn,hyper=warn,tantivy=warn,ureq=warn,ureq_proto=warn"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_log_filter_debug_level_and_suppresses_noisy_crates() {
        let filter = build_log_filter(LogLevel::Debug, 0);
        let rendered = filter.to_string();
        assert!(
            rendered.contains("debug"),
            "expected debug level in filter: {rendered}"
        );
        assert!(
            rendered.contains("rmcp::service=warn"),
            "missing rmcp::service=warn: {rendered}"
        );
        assert!(rendered.contains("h2=warn"), "missing h2=warn: {rendered}");
        assert!(
            rendered.contains("tantivy=warn"),
            "missing tantivy=warn: {rendered}"
        );
        assert!(
            rendered.contains("ureq=warn"),
            "missing ureq=warn: {rendered}"
        );
        assert!(
            rendered.contains("ureq_proto=warn"),
            "missing ureq_proto=warn: {rendered}"
        );
    }

    #[test]
    fn verbose_flag_ups_level_to_trace() {
        let filter = build_log_filter(LogLevel::Info, 2);
        let rendered = filter.to_string();
        assert!(
            rendered.contains("trace"),
            "expected trace level with -vv: {rendered}"
        );
    }

    #[test]
    fn single_verbose_sets_debug() {
        let filter = build_log_filter(LogLevel::Info, 1);
        let rendered = filter.to_string();
        assert!(
            rendered.contains("debug"),
            "expected debug level with -v: {rendered}"
        );
    }

    #[test]
    fn explicit_log_level_respected_without_verbose() {
        let filter = build_log_filter(LogLevel::Warn, 0);
        let rendered = filter.to_string();
        assert!(rendered.contains("warn"), "expected warn level: {rendered}");
    }
}
