use crate::config::Config;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::instrument;

const MAX_INPUT_LEN: usize = 10_240;

fn validate_not_empty(s: &str, name: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err(format!("'{}' must not be empty", name));
    }
    if s.len() > MAX_INPUT_LEN {
        return Err(format!(
            "'{}' too long ({} bytes, max {})",
            name,
            s.len(),
            MAX_INPUT_LEN
        ));
    }
    Ok(())
}

#[derive(Clone)]
pub struct MemoryHandler {
    conn: tracker::SparqlConnection,
}

impl MemoryHandler {
    pub fn new() -> Self {
        let conn = crate::tracker_db::init_db("./zakhor-db");
        Self { conn }
    }

    pub fn new_with_config(cfg: &Config) -> Self {
        let db_path = cfg.database.path.to_str().unwrap_or("./zakhor-db");
        let conn = crate::tracker_db::init_db(db_path);
        Self { conn }
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct StoreMemoryArgs {
    text: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ReadMemoryArgs {
    id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdateMemoryArgs {
    id: String,
    text: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct DeleteMemoryArgs {
    id: String,
}

#[tool_router(server_handler)]
impl MemoryHandler {
    #[tool(
        name = "store_memory",
        description = "Store knowledge in GNOME Tracker"
    )]
    #[instrument(skip(self, args))]
    async fn store_memory(&self, args: Parameters<StoreMemoryArgs>) -> Result<String, String> {
        validate_not_empty(&args.0.text, "text")?;
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            crate::tracker_db::store_memory(&this.conn, &args.0.text)
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    #[tool(
        name = "read_memory",
        description = "Read knowledge from GNOME Tracker by ID"
    )]
    #[instrument(skip(self, args))]
    async fn read_memory(&self, args: Parameters<ReadMemoryArgs>) -> Result<String, String> {
        validate_not_empty(&args.0.id, "id")?;
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            crate::tracker_db::read_memory(&this.conn, &args.0.id)
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    #[tool(
        name = "update_memory",
        description = "Update existing knowledge in GNOME Tracker"
    )]
    #[instrument(skip(self, args))]
    async fn update_memory(&self, args: Parameters<UpdateMemoryArgs>) -> Result<String, String> {
        validate_not_empty(&args.0.id, "id")?;
        validate_not_empty(&args.0.text, "text")?;
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            crate::tracker_db::update_memory(&this.conn, &args.0.id, &args.0.text)
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    #[tool(
        name = "delete_memory",
        description = "Delete knowledge from GNOME Tracker by ID"
    )]
    #[instrument(skip(self, args))]
    async fn delete_memory(&self, args: Parameters<DeleteMemoryArgs>) -> Result<String, String> {
        validate_not_empty(&args.0.id, "id")?;
        let id = args.0.id.clone();
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            crate::tracker_db::delete_memory(&this.conn, &id)?;
            Ok(format!("Deleted: {}", id))
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }
}
