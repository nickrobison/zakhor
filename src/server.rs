use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Clone)]
pub struct MemoryHandler {}

impl MemoryHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct StoreMemoryArgs {
    text: String,
}

#[tool_router(server_handler)]
impl MemoryHandler {
    #[tool(
        name = "store_memory",
        description = "Store knowledge in GNOME Tracker"
    )]
    async fn store_memory(&self, args: Parameters<StoreMemoryArgs>) -> Result<String, String> {
        Ok(format!("Stored: {}", args.0.text))
    }
}
