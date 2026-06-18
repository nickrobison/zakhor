use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod config;
mod error;
mod server;
mod tracker_db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
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
