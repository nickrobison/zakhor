use rmcp::ErrorData as McpError;
use rmcp::handler::server::ServerHandler;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams, Tool,
};
use rmcp::service::RequestContext;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracker::prelude::SparqlCursorExtManual;
use zakhor_common::config::Config;
use zakhor_model::extraction::{ExtractionConfig, ExtractionPipeline};
use zakhor_search::IndexSyncManager;

use crate::args::TripleResult;

#[cfg(test)]
use crate::args::{QueryEntitiesArgs, SearchHybridArgs};
#[cfg(test)]
use tracing::info_span;
#[cfg(test)]
use std::time::Instant;

pub(crate) fn args_hash<T: Serialize>(args: &T) -> String {
    let json = serde_json::to_string(args).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[derive(Clone)]
pub struct MemoryHandler {
    pub(crate) conn: tracker::SparqlConnection,
    pub sync_mgr: Option<Arc<IndexSyncManager>>,
    pub is_ephemeral: bool,
    extraction_init: Arc<OnceCell<Result<Arc<ExtractionPipeline>, String>>>,
}

impl MemoryHandler {
    /// Create a handler with a pre-existing Tracker connection.
    #[allow(dead_code)]
    pub fn with_connection(
        conn: tracker::SparqlConnection,
        sync_mgr: Option<Arc<IndexSyncManager>>,
    ) -> Self {
        Self {
            conn,
            sync_mgr,
            is_ephemeral: false,
            extraction_init: Arc::new(OnceCell::new()),
        }
    }

    #[tracing::instrument(skip(cfg, sync_mgr))]
    pub fn new_with_config(
        cfg: &Config,
        sync_mgr: Option<Arc<IndexSyncManager>>,
        is_ephemeral: bool,
    ) -> Self {
        let db_path = cfg.database.path.to_str().unwrap_or("./zakhor-db");
        let conn = zakhor_storage::tracker_db::init_db(db_path);

        let extraction_cfg = ExtractionConfig {
            model_path: cfg.extraction.model_path.clone(),
            tokenizer_path: cfg.extraction.tokenizer_path.clone(),
            entity_labels: cfg.extraction.entity_labels.clone(),
            relation_labels: cfg.extraction.relation_labels.clone(),
            entity_threshold: cfg.extraction.entity_threshold,
            relation_threshold: cfg.extraction.relation_threshold,
        };
        let model_dir = cfg.extraction.model_dir.clone();
        let extraction_init: Arc<OnceCell<Result<Arc<ExtractionPipeline>, String>>> =
            Arc::new(OnceCell::new());
        let extraction_bg = extraction_init.clone();
        tokio::task::spawn_blocking(move || {
            tracing::info!("Starting extraction model download from HF in background");
            let result = match ExtractionPipeline::new_with_setup(extraction_cfg, &model_dir) {
                Ok(pipeline) => {
                    tracing::info!("Extraction model ready in {}", model_dir.display());
                    Ok(Arc::new(pipeline))
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to set up extraction model — extraction disabled"
                    );
                    Err(format!("Extraction pipeline init failed: {e}"))
                }
            };
            let _ = extraction_bg.set(result);
        });

        Self {
            conn,
            sync_mgr,
            is_ephemeral,
            extraction_init,
        }
    }

    pub async fn ensure_extraction(&self) -> Result<Arc<ExtractionPipeline>, String> {
        if let Some(result) = self.extraction_init.get() {
            return result.clone();
        }
        Err("Extraction model not initialized, see logs".to_string())
    }

    pub fn api_state(&self) -> crate::api::ApiState {
        crate::api::ApiState::new(self.conn.clone(), self.sync_mgr.clone())
    }

    /// Combine all per-tool routers into a single router for the MCP handler.
    pub fn tool_router() -> ToolRouter<MemoryHandler> {
        crate::tools::tool_router()
    }
}

pub fn is_resource_iri(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://") || s.starts_with("urn:")
}

pub fn query_depth1(
    conn: &tracker::SparqlConnection,
    start_id: &str,
    edge_types: &[String],
) -> Result<Vec<TripleResult>, String> {
    let sparql = crate::tools::build_traverse_query(start_id, 1, edge_types);
    let cursor = conn
        .query(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| format!("Query failed: {e}"))?;

    let mut triples = Vec::new();
    loop {
        match cursor.next(None::<&gio::Cancellable>) {
            Ok(true) => {
                let s = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
                let p = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
                let o = cursor.string(2).map(|s| s.to_string()).unwrap_or_default();
                triples.push(TripleResult {
                    subject: s,
                    predicate: p,
                    object: o,
                });
            }
            Ok(false) => break,
            Err(e) => return Err(format!("Cursor error: {e}")),
        }
    }
    Ok(triples)
}

#[rmcp::tool_handler(router = Self::tool_router())]
impl ServerHandler for MemoryHandler {
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let mut tools = Self::tool_router().list_all();
        if !self.is_ephemeral {
            tools.retain(|t| !t.name.starts_with("admin_"));
        }
        Ok(ListToolsResult {
            tools,
            meta: None,
            next_cursor: None,
        })
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        if !self.is_ephemeral && name.starts_with("admin_") {
            return None;
        }
        Self::tool_router().get(name).cloned()
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        if !self.is_ephemeral && request.name.starts_with("admin_") {
            return Err(McpError::invalid_params("tool not found", None));
        }
        let tcc = ToolCallContext::new(self, request, context);
        Self::tool_router().call(tcc).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_hash_deterministic() {
        let args = QueryEntitiesArgs {
            pattern: "hello".into(),
            limit: 10,
        };
        let h1 = args_hash(&args);
        let h2 = args_hash(&args);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16, "hash should be 16 hex chars");
    }

    #[test]
    fn test_args_hash_different_args_differ() {
        let a = QueryEntitiesArgs {
            pattern: "foo".into(),
            limit: 10,
        };
        let b = QueryEntitiesArgs {
            pattern: "bar".into(),
            limit: 10,
        };
        assert_ne!(args_hash(&a), args_hash(&b));
    }

    #[test]
    fn test_args_hash_different_types_differ() {
        let store = QueryEntitiesArgs {
            pattern: "x".into(),
            limit: 10,
        };
        let read = SearchHybridArgs {
            query: "x".into(),
            limit: 10,
        };
        assert_ne!(args_hash(&store), args_hash(&read));
    }

    #[test]
    fn test_correlation_id_unique() {
        let id1 = crate::new_correlation_id();
        let id2 = crate::new_correlation_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("mcp-"));
        assert!(id2.starts_with("mcp-"));
    }

    #[test]
    fn test_correlation_id_monotonic() {
        let id1 = crate::new_correlation_id();
        let id2 = crate::new_correlation_id();
        assert!(
            id1 < id2,
            "correlation IDs should increase: {} < {}",
            id1,
            id2
        );
    }

    #[test]
    fn test_mcp_tool_span_has_required_fields() {
        use std::sync::{Arc, Mutex};
        use tracing::span::{Attributes, Id};
        use tracing::subscriber::with_default;
        use tracing::{Event, Metadata, Subscriber};

        #[derive(Default, Clone)]
        struct CaptureSub {
            new_span_fields: Arc<Mutex<Vec<String>>>,
            recorded_fields: Arc<Mutex<Vec<String>>>,
        }

        impl Subscriber for CaptureSub {
            fn enabled(&self, _: &Metadata<'_>) -> bool {
                true
            }

            fn new_span(&self, attrs: &Attributes<'_>) -> Id {
                let mut fields = self.new_span_fields.lock().unwrap();
                let mut visitor = FieldCapture(&mut fields);
                attrs.record(&mut visitor);
                Id::from_u64(1)
            }

            fn record(&self, _: &Id, record: &tracing::span::Record<'_>) {
                let mut fields = self.recorded_fields.lock().unwrap();
                let mut visitor = FieldCapture(&mut fields);
                record.record(&mut visitor);
            }

            fn record_follows_from(&self, _: &Id, _: &Id) {}
            fn event(&self, _: &Event<'_>) {}
            fn enter(&self, _: &Id) {}
            fn exit(&self, _: &Id) {}
        }

        struct FieldCapture<'a>(&'a mut Vec<String>);

        impl tracing::field::Visit for FieldCapture<'_> {
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                self.0.push(format!("{}={}", field.name(), value));
            }
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                self.0.push(format!("{}={:?}", field.name(), value));
            }
            fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
                self.0.push(format!("{}={:?}", field.name(), value));
            }
            fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
                self.0.push(format!("{}={}", field.name(), value));
            }
            fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
                self.0.push(format!("{}={}", field.name(), value));
            }
            fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
                self.0.push(format!("{}={}", field.name(), value));
            }
        }

        let sub = CaptureSub::default();
        let create_fields = sub.new_span_fields.clone();
        let rec_fields = sub.recorded_fields.clone();

        with_default(sub, || {
            let span = info_span!(
                "mcp_tool",
                tool = "test_tool",
                correlation_id = "mcp-000001",
                args_hash = "abc123",
                duration_ms = tracing::field::Empty,
                result = tracing::field::Empty,
            );
            span.record("duration_ms", 1.5f64);
            span.record("result", "success");
        });

        let created = create_fields.lock().unwrap();
        let created_all = created.join(" | ");
        assert!(
            created_all.contains("tool=test_tool"),
            "should contain tool field at create: {}",
            created_all
        );
        assert!(
            created_all.contains("correlation_id=mcp-000001"),
            "should contain correlation_id at create: {}",
            created_all
        );
        assert!(
            created_all.contains("args_hash=abc123"),
            "should contain args_hash at create: {}",
            created_all
        );

        let recorded = rec_fields.lock().unwrap();
        let recorded_all = recorded.join(" | ");
        assert!(
            recorded_all.contains("duration_ms=1.5"),
            "should record duration_ms: {}",
            recorded_all
        );
        assert!(
            recorded_all.contains("result=success"),
            "should record result: {}",
            recorded_all
        );
    }
}
