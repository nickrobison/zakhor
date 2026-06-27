use rmcp::ErrorData as McpError;
use rmcp::handler::server::ServerHandler;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams, Tool,
};
use rmcp::service::RequestContext;
use rmcp::tool;
use rmcp::tool_router;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::info_span;
use tracker::prelude::SparqlCursorExtManual;
use zakhor_common::config::Config;
use zakhor_model::decision::{CreateDecisionArgs, DecisionModel};
use zakhor_model::extraction::{ExtractionConfig, ExtractionPipeline};
use zakhor_model::ingestion::{IngestionPipeline, StoreObservationArgs};
use zakhor_search::IndexSyncManager;

use crate::tool_capture;
use crate::tools;

fn args_hash<T: Serialize>(args: &T) -> String {
    let json = serde_json::to_string(args).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[derive(Clone)]
pub struct MemoryHandler {
    conn: tracker::SparqlConnection,
    pub sync_mgr: Option<Arc<Mutex<IndexSyncManager>>>,
    pub is_ephemeral: bool,
    pub extraction: Option<Arc<ExtractionPipeline>>,
}

impl MemoryHandler {
    /// Create a handler with a pre-existing Tracker connection.
    #[allow(dead_code)]
    pub fn with_connection(
        conn: tracker::SparqlConnection,
        sync_mgr: Option<Arc<Mutex<IndexSyncManager>>>,
    ) -> Self {
        Self {
            conn,
            sync_mgr,
            is_ephemeral: false,
            extraction: None,
        }
    }

    pub fn new_with_config(
        cfg: &Config,
        sync_mgr: Option<Arc<Mutex<IndexSyncManager>>>,
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
        let extraction = if !extraction_cfg.model_path.as_os_str().is_empty() {
            Some(Arc::new(ExtractionPipeline::new(extraction_cfg)))
        } else {
            None
        };
        Self {
            conn,
            sync_mgr,
            is_ephemeral,
            extraction,
        }
    }

    pub fn api_state(&self) -> crate::api::ApiState {
        crate::api::ApiState::new(self.conn.clone(), self.sync_mgr.clone())
    }
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct RebuildIndexesArgs {}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct QueryEntitiesArgs {
    pub pattern: String,
    pub limit: u32,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct TraverseGraphArgs {
    pub start_id: String,
    pub depth: u32,
    pub edge_types: Vec<String>,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct SearchHybridArgs {
    pub query: String,
    pub limit: u32,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct RecordDecisionArgs {
    pub context: String,
    pub decision: String,
    pub alternatives: Vec<String>,
    pub rationale: String,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct ExtractAndStoreArgs {
    pub uri: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct ExtractAndStoreResponse {
    pub observation_uri: String,
    pub entity_count: u64,
    pub relation_count: u64,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct StoreObservationResponse {
    pub observation_uri: String,
    pub triple_count: u64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct EntityResult {
    pub uri: String,
    pub label: String,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct QueryEntitiesResponse {
    pub entities: Vec<EntityResult>,
    pub count: u64,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct TripleResult {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// Check if a string looks like a resource IRI (not an internal Tracker ID or literal).
pub fn is_resource_iri(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://") || s.starts_with("urn:")
}

/// Execute a depth-1 SPARQL traversal query and return the result triples.
pub fn query_depth1(
    conn: &tracker::SparqlConnection,
    start_id: &str,
    edge_types: &[String],
) -> Result<Vec<TripleResult>, String> {
    let sparql = tools::build_traverse_query(start_id, 1, edge_types);
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

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct TraverseGraphResponse {
    pub triples: Vec<TripleResult>,
    pub count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct SearchResult {
    pub id: String,
    pub score: f64,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct SearchHybridResponse {
    pub results: Vec<SearchResult>,
    pub count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct RecordDecisionResponse {
    pub decision_uri: String,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct AdminInjectToolCallArgs {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub session_id: String,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct AdminInjectToolCallResponse {
    pub uri: String,
}

#[tool_router]
impl MemoryHandler {
    #[tool(
        description = "Store an observation about entities and relations in the knowledge graph"
    )]
    async fn store_observation(
        &self,
        Parameters(args): Parameters<StoreObservationArgs>,
    ) -> Result<Json<StoreObservationResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "store_observation",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let result = (|| -> Result<Json<StoreObservationResponse>, String> {
            let text = args.text.clone();
            let entity_uris: Vec<String> = args.entities.iter().map(|e| e.uri.clone()).collect();

            let mut pipeline = IngestionPipeline::new();
            let ingest_result = pipeline
                .ingest(&self.conn, args)
                .map_err(|e| format!("Ingest failed: {e}"))?;

            if let Some(ref sync_mgr) = self.sync_mgr {
                let mgr = sync_mgr.lock().expect("sync manager lock poisoned");
                if let Err(e) =
                    mgr.sync_observation(&ingest_result.observation_uri, &text, &entity_uris)
                {
                    tracing::warn!(error = %e, "Failed to sync observation to indexes");
                }
            }

            Ok(Json(StoreObservationResponse {
                observation_uri: ingest_result.observation_uri,
                triple_count: ingest_result.triple_count as u64,
            }))
        })();

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }

    #[tool(description = "Query entities by label pattern in the knowledge graph")]
    async fn query_entities(
        &self,
        Parameters(args): Parameters<QueryEntitiesArgs>,
    ) -> Result<Json<QueryEntitiesResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "query_entities",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let result = (|| -> Result<Json<QueryEntitiesResponse>, String> {
            let sparql = tools::build_entity_query(&args.pattern, args.limit);
            let cursor = self
                .conn
                .query(&sparql, None::<&gio::Cancellable>)
                .map_err(|e| format!("SPARQL query failed: {e}"))?;

            let mut entities: Vec<EntityResult> = Vec::new();
            while cursor
                .next(None::<&gio::Cancellable>)
                .map_err(|e| format!("Cursor error: {e}"))?
            {
                let uri = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
                let label = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
                entities.push(EntityResult { uri, label });
            }

            let count = entities.len() as u64;
            Ok(Json(QueryEntitiesResponse { entities, count }))
        })();

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }

    #[tool(description = "Traverse the knowledge graph from a starting node")]
    async fn traverse_graph(
        &self,
        Parameters(args): Parameters<TraverseGraphArgs>,
    ) -> Result<Json<TraverseGraphResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "traverse_graph",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let result = (|| -> Result<Json<TraverseGraphResponse>, String> {
            if args.depth <= 1 {
                // depth=1: single SPARQL query works correctly
                let sparql =
                    tools::build_traverse_query(&args.start_id, args.depth, &args.edge_types);
                match self.conn.query(&sparql, None::<&gio::Cancellable>) {
                    Ok(cursor) => {
                        let mut triples: Vec<TripleResult> = Vec::new();
                        loop {
                            match cursor.next(None::<&gio::Cancellable>) {
                                Ok(true) => {
                                    let s =
                                        cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
                                    let p =
                                        cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
                                    let o =
                                        cursor.string(2).map(|s| s.to_string()).unwrap_or_default();
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
                        let count = triples.len() as u64;
                        Ok(Json(TraverseGraphResponse {
                            triples,
                            count,
                            warning: None,
                        }))
                    }
                    Err(e) => Ok(Json(TraverseGraphResponse {
                        triples: vec![],
                        count: 0,
                        warning: Some(format!("Query issue: {e}")),
                    })),
                }
            } else {
                // depth>1: iterative application-level traversal avoids Tracker SPARQL
                // variable chaining bug where IRIs bound in the first hop cannot be
                // matched against internal numeric IDs in subsequent hops.
                let mut all_triples: Vec<TripleResult> = Vec::new();
                let mut seen_sop: HashSet<(String, String, String)> = HashSet::new();
                let mut visited_iris: HashSet<String> = HashSet::new();
                let mut frontier: Vec<String> = vec![args.start_id.clone()];
                visited_iris.insert(args.start_id.clone());

                for _ in 0..args.depth {
                    let mut next_frontier: Vec<String> = Vec::new();
                    for node in &frontier {
                        let triples = query_depth1(&self.conn, node, &args.edge_types)?;
                        for triple in &triples {
                            let key = (
                                triple.subject.clone(),
                                triple.predicate.clone(),
                                triple.object.clone(),
                            );
                            if seen_sop.insert(key) {
                                all_triples.push(triple.clone());
                            }
                            // Collect new IRIs from both subject and object for next frontier
                            let obj_iri = is_resource_iri(&triple.object)
                                && visited_iris.insert(triple.object.clone());
                            if obj_iri {
                                next_frontier.push(triple.object.clone());
                            }
                            let subj_iri = is_resource_iri(&triple.subject)
                                && visited_iris.insert(triple.subject.clone());
                            if subj_iri {
                                next_frontier.push(triple.subject.clone());
                            }
                        }
                    }
                    frontier = next_frontier;
                    if frontier.is_empty() {
                        break;
                    }
                }

                let count = all_triples.len() as u64;
                Ok(Json(TraverseGraphResponse {
                    triples: all_triples,
                    count,
                    warning: None,
                }))
            }
        })();

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }

    #[tool(description = "Hybrid search across lexical and semantic indexes using RRF fusion")]
    async fn search_hybrid(
        &self,
        Parameters(args): Parameters<SearchHybridArgs>,
    ) -> Result<Json<SearchHybridResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "search_hybrid",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let result = match self.sync_mgr {
            Some(ref sync_mgr) => {
                let mgr = sync_mgr.lock().expect("sync manager lock poisoned");
                let results = tools::hybrid_search(
                    &mgr.lexical,
                    &mgr.semantic,
                    &args.query,
                    args.limit as usize,
                );
                let docs: Vec<SearchResult> = results
                    .into_iter()
                    .map(|d| SearchResult {
                        id: d.id,
                        score: d.score,
                    })
                    .collect();
                let count = docs.len() as u64;
                Ok(Json(SearchHybridResponse {
                    results: docs,
                    count,
                    warning: None,
                }))
            }
            None => Ok(Json(SearchHybridResponse {
                results: vec![],
                count: 0,
                warning: Some("Indexes not available".to_string()),
            })),
        };

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }

    #[tool(
        description = "Record a decision with context, alternatives, and rationale in the knowledge graph"
    )]
    async fn record_decision(
        &self,
        Parameters(args): Parameters<RecordDecisionArgs>,
    ) -> Result<Json<RecordDecisionResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "record_decision",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let result = (|| -> Result<Json<RecordDecisionResponse>, String> {
            let decision_args = CreateDecisionArgs {
                context: args.context,
                outcome: args.decision,
                alternatives: args.alternatives,
                rationale: args.rationale,
                affects: vec![],
                derived_from: vec![],
                supersedes: None,
                conflicts_with: vec![],
                depends_on: vec![],
                project_uri: None,
            };
            let create_result = DecisionModel::create(&self.conn, decision_args)?;

            Ok(Json(RecordDecisionResponse {
                decision_uri: create_result.decision_uri.as_str().to_string(),
            }))
        })();

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }

    #[tool(
        description = "Extract entities and relations from text and store them in the knowledge graph"
    )]
    async fn extract_and_store(
        &self,
        Parameters(args): Parameters<ExtractAndStoreArgs>,
    ) -> Result<Json<ExtractAndStoreResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "extract_and_store",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let extraction = self
            .extraction
            .as_ref()
            .ok_or_else(|| "Extraction pipeline not configured. Set model_path in [extraction] section of zakhor.toml.".to_string())?;

        let mut pipeline = IngestionPipeline::new();
        let ingest_result = pipeline
            .extract_and_ingest(&self.conn, &args.text, extraction)
            .await
            .map_err(|e| format!("Extract and ingest failed: {e}"))?;

        if let Some(ref sync_mgr) = self.sync_mgr {
            let mgr = sync_mgr.lock().expect("sync manager lock poisoned");
            if let Err(e) = mgr.sync_observation(
                &ingest_result.observation_uri,
                &args.text,
                &[],
            ) {
                tracing::warn!(error = %e, "Failed to sync extracted observation to indexes");
            }
        }

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", "success");
        span.record("duration_ms", duration_ms);

        Ok(Json(ExtractAndStoreResponse {
            observation_uri: ingest_result.observation_uri,
            entity_count: 0,
            relation_count: 0,
        }))
    }

    #[tool(
        name = "rebuild_indexes",
        description = "Rebuild all search indexes from Tracker"
    )]
    async fn rebuild_indexes(
        &self,
        Parameters(_args): Parameters<RebuildIndexesArgs>,
    ) -> Result<String, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "rebuild_indexes",
            correlation_id = &crate::new_correlation_id(),
            args_hash = %args_hash(&_args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let start = Instant::now();

        let propagate_span = span.clone();
        let this = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _guard = propagate_span.enter();
            match &this.sync_mgr {
                Some(mgr) => match mgr.lock() {
                    Ok(guard) => guard
                        .rebuild_all(&this.conn)
                        .map_err(|e| format!("Rebuild failed: {e}"))
                        .map(|_| "Indexes rebuilt successfully".to_string()),
                    Err(e) => Err(format!("Sync manager lock poisoned: {e}")),
                },
                None => Err("No sync manager available (indexes disabled)".to_string()),
            }
        })
        .await
        .map_err(|e| format!("Task join error: {e}"))?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }

    // ── Admin tools (ephemeral-only) ────────────────────────────────────

    #[tool(
        name = "admin_rebuild_indexes",
        description = "[Admin] Rebuild all search indexes from Tracker (ephemeral mode only)"
    )]
    async fn admin_rebuild_indexes(
        &self,
        Parameters(_args): Parameters<RebuildIndexesArgs>,
    ) -> Result<String, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "admin_rebuild_indexes",
            correlation_id = &crate::new_correlation_id(),
            args_hash = %args_hash(&_args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let start = Instant::now();

        let propagate_span = span.clone();
        let this = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _guard = propagate_span.enter();
            match &this.sync_mgr {
                Some(mgr) => match mgr.lock() {
                    Ok(guard) => guard
                        .rebuild_all(&this.conn)
                        .map_err(|e| format!("Rebuild failed: {e}"))
                        .map(|_| "Indexes rebuilt successfully".to_string()),
                    Err(e) => Err(format!("Sync manager lock poisoned: {e}")),
                },
                None => Err("No sync manager available (indexes disabled)".to_string()),
            }
        })
        .await
        .map_err(|e| format!("Task join error: {e}"))?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }

    #[tool(
        name = "admin_inject_tool_call",
        description = "[Admin] Inject a ToolCall node into the knowledge graph (ephemeral mode only)"
    )]
    async fn admin_inject_tool_call(
        &self,
        Parameters(args): Parameters<AdminInjectToolCallArgs>,
    ) -> Result<Json<AdminInjectToolCallResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "admin_inject_tool_call",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let result = (|| -> Result<Json<AdminInjectToolCallResponse>, String> {
            let uri = tool_capture::capture_tool_call(
                &self.conn,
                &args.tool_name,
                &args.arguments,
                &args.session_id,
            )?;
            Ok(Json(AdminInjectToolCallResponse { uri }))
        })();

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }
}

// ── ServerHandler impl (filters admin tools when not ephemeral) ─────────

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
            // Exercise late-binding field recording (the real tool
            // methods call span.record for duration_ms and result).
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
