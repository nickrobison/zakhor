use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};

mod server;
mod tracker_db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _conn = tracker_db::init_db("./zakhor-db");

    let service = server::MemoryHandler::new();
    let transport = (stdin(), stdout());

    let server = service.serve(transport).await?;
    server.waiting().await?;

    Ok(())
}
