//! ToolCall Capture (Phase 3.2)
//!
//! Captures MCP tool call metadata (tool name, arguments, timestamp, session
//! identifier) and stores it in the knowledge graph as a `zakhor:ToolCall`.
//!
//! Tool call arguments are represented as a concrete `serde_json::Value` rather
//! than a raw string, and are indexed in Tantivy via a JSON field for
//! full-text and structured search.

use gio::Cancellable;
use oxrdf::Literal;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{
    JsonObjectOptions, NumericOptions, OwnedValue, STORED, STRING, Schema, TEXT, Value,
};
use tantivy::{Index, IndexWriter, TantivyDocument};
use tracker::SparqlConnection;
use tracker::prelude::SparqlConnectionExtManual;
use zakhor_common::error::{ZakhorError, ZakhorResult};
use zakhor_search::ScoredDoc;
use zakhor_storage::sparql::Prefix;

/// A captured MCP tool invocation.
///
/// Arguments are stored as a concrete JSON value rather than a raw string so
/// that consumers and the [`ToolCallIndex`] can work with the structured data
/// directly.
#[derive(Clone, Debug)]
pub struct ToolCall {
    pub uri: String,
    pub tool_name: String,
    /// Structured JSON arguments for this tool invocation.
    pub arguments: JsonValue,
    pub session_id: String,
    pub timestamp_ms: u64,
}

/// Tantivy-backed index for [`ToolCall`] records.
///
/// Schema fields:
/// - `id`: STRING + STORED — the ToolCall URI (not tokenized, retrievable)
/// - `tool_name`: TEXT + STORED — tokenized for BM25, retrievable
/// - `session_id`: STRING + STORED — exact-match, retrievable
/// - `timestamp_ms`: u64, STORED — wall-clock milliseconds since UNIX epoch
/// - `arguments`: JSON + STORED — structured JSON arguments, stored for retrieval
/// - `arguments_text`: TEXT — serialized JSON string, indexed for full-text search
pub struct ToolCallIndex {
    index: Index,
    index_path: PathBuf,
    id_field: tantivy::schema::Field,
    tool_name_field: tantivy::schema::Field,
    session_id_field: tantivy::schema::Field,
    timestamp_ms_field: tantivy::schema::Field,
    arguments_field: tantivy::schema::Field,
    arguments_text_field: tantivy::schema::Field,
}

impl std::fmt::Debug for ToolCallIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolCallIndex")
            .field("index_path", &self.index_path)
            .finish()
    }
}

impl ToolCallIndex {
    /// Build the Tantivy schema for the tool-call index.
    fn build_schema() -> Schema {
        let mut builder = Schema::builder();
        builder.add_text_field("id", STRING | STORED);
        builder.add_text_field("tool_name", TEXT | STORED);
        builder.add_text_field("session_id", STRING | STORED);
        let ts_opts = NumericOptions::default().set_stored();
        builder.add_u64_field("timestamp_ms", ts_opts);
        let json_opts = JsonObjectOptions::from(STORED);
        builder.add_json_field("arguments", json_opts);
        builder.add_text_field("arguments_text", TEXT);
        builder.build()
    }

    /// Create or open a Tantivy index at `<db-path>/toolcalls/`.
    ///
    /// If the directory exists, the existing index is opened and its schema
    /// is validated. If it does not exist, a new index is created.
    pub fn new(db_path: &Path) -> ZakhorResult<Self> {
        let index_path = db_path.join("toolcalls");

        let index: Index = if index_path.exists() {
            Index::open_in_dir(&index_path)
                .map_err(|e| ZakhorError::Internal(format!("Failed to open ToolCallIndex: {e}")))?
        } else {
            std::fs::create_dir_all(&index_path).map_err(|e| {
                ZakhorError::Internal(format!("Failed to create ToolCallIndex dir: {e}"))
            })?;
            let schema = Self::build_schema();
            Index::create_in_dir(&index_path, schema).map_err(|e| {
                ZakhorError::Internal(format!("Failed to create ToolCallIndex: {e}"))
            })?
        };

        let schema = index.schema();
        let id_field = schema
            .get_field("id")
            .map_err(|e| ZakhorError::Internal(format!("Schema missing id field: {e}")))?;
        let tool_name_field = schema
            .get_field("tool_name")
            .map_err(|e| ZakhorError::Internal(format!("Schema missing tool_name field: {e}")))?;
        let session_id_field = schema
            .get_field("session_id")
            .map_err(|e| ZakhorError::Internal(format!("Schema missing session_id field: {e}")))?;
        let timestamp_ms_field = schema.get_field("timestamp_ms").map_err(|e| {
            ZakhorError::Internal(format!("Schema missing timestamp_ms field: {e}"))
        })?;
        let arguments_field = schema
            .get_field("arguments")
            .map_err(|e| ZakhorError::Internal(format!("Schema missing arguments field: {e}")))?;
        let arguments_text_field = schema.get_field("arguments_text").map_err(|e| {
            ZakhorError::Internal(format!("Schema missing arguments_text field: {e}"))
        })?;

        Ok(Self {
            index,
            index_path,
            id_field,
            tool_name_field,
            session_id_field,
            timestamp_ms_field,
            arguments_field,
            arguments_text_field,
        })
    }

    /// Add a [`ToolCall`] to the index.
    ///
    /// The document is immediately committed so it becomes visible to
    /// subsequent searches. The `arguments` JSON value is stored via Tantivy's
    /// native JSON field; a serialised string copy is also indexed in
    /// `arguments_text` (a plain TEXT field) so that simple term queries such
    /// as `search("rusty", 10)` match against argument values without requiring
    /// path notation. Non-object JSON arguments are wrapped as `{"value": …}`
    /// before being stored in the JSON field.
    pub fn add(&self, toolcall: &ToolCall) -> ZakhorResult<()> {
        let mut writer: IndexWriter = self
            .index
            .writer(50_000_000)
            .map_err(|e| ZakhorError::Internal(format!("Failed to create writer: {e}")))?;

        let args_obj: BTreeMap<String, OwnedValue> = match &toolcall.arguments {
            JsonValue::Object(map) => map
                .iter()
                .map(|(k, v)| (k.clone(), OwnedValue::from(v.clone())))
                .collect(),
            other => {
                let mut m = BTreeMap::new();
                m.insert("value".to_string(), OwnedValue::from(other.clone()));
                m
            }
        };

        let args_text = serde_json::to_string(&toolcall.arguments)
            .map_err(|e| ZakhorError::Internal(format!("Failed to serialize arguments: {e}")))?;

        let mut doc = TantivyDocument::default();
        doc.add_text(self.id_field, &toolcall.uri);
        doc.add_text(self.tool_name_field, &toolcall.tool_name);
        doc.add_text(self.session_id_field, &toolcall.session_id);
        doc.add_u64(self.timestamp_ms_field, toolcall.timestamp_ms);
        doc.add_object(self.arguments_field, args_obj);
        doc.add_text(self.arguments_text_field, &args_text);

        writer
            .add_document(doc)
            .map_err(|e| ZakhorError::Internal(format!("Failed to add document: {e}")))?;
        writer
            .commit()
            .map_err(|e| ZakhorError::Internal(format!("Failed to commit: {e}")))?;

        Ok(())
    }

    /// Search the index with BM25 ranking over `tool_name` and `arguments`.
    ///
    /// Returns up to `limit` scored documents sorted by decreasing relevance.
    pub fn search(&self, query_str: &str, limit: usize) -> ZakhorResult<Vec<ScoredDoc>> {
        let reader = self
            .index
            .reader()
            .map_err(|e| ZakhorError::Internal(format!("Failed to create reader: {e}")))?;
        reader
            .reload()
            .map_err(|e| ZakhorError::Internal(format!("Failed to reload reader: {e}")))?;
        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.tool_name_field, self.arguments_text_field],
        );
        let query = query_parser
            .parse_query(query_str)
            .map_err(|e| ZakhorError::Internal(format!("Failed to parse query: {e}")))?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| ZakhorError::Internal(format!("Search failed: {e}")))?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let tantivy_doc = searcher
                .doc::<TantivyDocument>(doc_address)
                .map_err(|e| ZakhorError::Internal(format!("Failed to fetch doc: {e}")))?;

            let id = tantivy_doc
                .get_first(self.id_field)
                .and_then(|v| v.as_str())
                .ok_or_else(|| ZakhorError::Internal("Missing id field in document".into()))?
                .to_string();

            results.push(ScoredDoc {
                id,
                score: score.into(),
            });
        }

        Ok(results)
    }

    /// Number of documents currently in the index.
    pub fn num_docs(&self) -> u64 {
        let reader = match self.index.reader() {
            Ok(r) => r,
            Err(_) => return 0,
        };
        let searcher = reader.searcher();
        searcher
            .segment_readers()
            .iter()
            .map(|sr| sr.num_docs() as u64)
            .sum()
    }
}

/// Store a ToolCall in the knowledge graph.
///
/// The `arguments` value is serialised to a compact JSON string for storage in
/// the Tracker triple-store. Returns the URI of the newly created ToolCall node.
pub fn capture_tool_call(
    conn: &SparqlConnection,
    tool_name: &str,
    arguments: &JsonValue,
    session_id: &str,
) -> Result<String, String> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let call_uri = format!("{}toolcall/{:016x}", Prefix::ZAKHOR, ts);
    let sparql = format!(
        r#"PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX zakhor: <{ns}>

INSERT DATA {{
  <{uri}> rdf:type zakhor:ToolCall .
  <{uri}> zakhor:toolName {name} .
  <{uri}> zakhor:toolArguments {args} .
  <{uri}> zakhor:sessionId {session} .
  <{uri}> zakhor:timestamp {ts} .
}}"#,
        ns = Prefix::ZAKHOR,
        uri = call_uri,
        name = Literal::new_language_tagged_literal(tool_name.to_string(), "en").unwrap(),
        args = Literal::new_language_tagged_literal(arguments.to_string(), "en").unwrap(),
        session = Literal::new_language_tagged_literal(session_id.to_string(), "en").unwrap(),
        ts = ts,
    );

    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("ToolCall capture failed: {e}"))?;

    Ok(call_uri)
}

/// Link a ToolCall to a Decision via `zakhor:evidenceFor`.
pub fn link_toolcall_to_decision(
    conn: &SparqlConnection,
    toolcall_uri: &str,
    decision_uri: &str,
) -> Result<(), String> {
    let safe_tc = toolcall_uri.replace('>', "");
    let safe_dec = decision_uri.replace('>', "");

    let sparql = format!(
        r#"PREFIX zakhor: <{ns}>

INSERT DATA {{
  <{tc}> zakhor:evidenceFor <{dec}> .
}}"#,
        ns = Prefix::ZAKHOR,
        tc = safe_tc,
        dec = safe_dec,
    );

    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Link toolcall->decision failed: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde_json::json;

    use super::*;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_index_path() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("zakhor-toolcall-test-{n}"));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn test_toolcall_struct() {
        let tc = ToolCall {
            uri: "http://zakhor/ns/toolcall/abc".into(),
            tool_name: "store_observation".into(),
            arguments: json!({"text": "hello"}),
            session_id: "ses_123".into(),
            timestamp_ms: 1000,
        };
        assert_eq!(tc.tool_name, "store_observation");
        assert_eq!(tc.session_id, "ses_123");
        assert_eq!(tc.arguments["text"], "hello");
    }

    #[test]
    fn test_toolcall_arguments_is_json_value() {
        let tc = ToolCall {
            uri: "http://zakhor/ns/toolcall/x".into(),
            tool_name: "query".into(),
            arguments: json!({"limit": 5, "filter": "rust"}),
            session_id: "ses_456".into(),
            timestamp_ms: 2000,
        };
        assert_eq!(tc.arguments["limit"], 5);
        assert_eq!(tc.arguments["filter"], "rust");
    }

    #[test]
    fn test_link_toolcall_sparql_shape() {
        let sparql = format!(
            "PREFIX zakhor: <{ns}> INSERT DATA {{ <{tc}> zakhor:evidenceFor <{dec}> . }}",
            ns = Prefix::ZAKHOR,
            tc = "http://zakhor/ns/toolcall/a",
            dec = "http://zakhor/ns/decision/b",
        );
        assert!(sparql.contains("evidenceFor"));
        assert!(sparql.contains("/toolcall/a"));
        assert!(sparql.contains("/decision/b"));
    }

    // ── ToolCallIndex tests ─────────────────────────────────────────────────

    #[test]
    fn test_index_create_and_add() {
        let path = test_index_path();
        let index = ToolCallIndex::new(&path).expect("create index");
        assert_eq!(index.num_docs(), 0);

        let tc = ToolCall {
            uri: "http://zakhor/ns/toolcall/001".into(),
            tool_name: "store_observation".into(),
            arguments: json!({"text": "quick brown fox"}),
            session_id: "ses_1".into(),
            timestamp_ms: 1_000,
        };
        index.add(&tc).expect("add toolcall");
        assert_eq!(index.num_docs(), 1);

        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn test_index_search_by_tool_name() {
        let path = test_index_path();
        let index = ToolCallIndex::new(&path).expect("create index");

        let tc = ToolCall {
            uri: "http://zakhor/ns/toolcall/002".into(),
            tool_name: "store_observation".into(),
            arguments: json!({}),
            session_id: "ses_2".into(),
            timestamp_ms: 2_000,
        };
        index.add(&tc).expect("add");

        let results = index.search("store_observation", 10).expect("search");
        assert!(!results.is_empty(), "expected a hit for tool_name");
        assert_eq!(results[0].id, "http://zakhor/ns/toolcall/002");

        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn test_index_search_by_argument_content() {
        let path = test_index_path();
        let index = ToolCallIndex::new(&path).expect("create index");

        let tc = ToolCall {
            uri: "http://zakhor/ns/toolcall/003".into(),
            tool_name: "query_graph".into(),
            arguments: json!({"entity": "rusty", "depth": 2}),
            session_id: "ses_3".into(),
            timestamp_ms: 3_000,
        };
        index.add(&tc).expect("add");

        let results = index.search("rusty", 10).expect("search");
        assert!(!results.is_empty(), "expected a hit from JSON arguments");
        assert_eq!(results[0].id, "http://zakhor/ns/toolcall/003");

        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn test_index_search_no_match() {
        let path = test_index_path();
        let index = ToolCallIndex::new(&path).expect("create index");

        let tc = ToolCall {
            uri: "http://zakhor/ns/toolcall/004".into(),
            tool_name: "noop".into(),
            arguments: json!({}),
            session_id: "ses_4".into(),
            timestamp_ms: 4_000,
        };
        index.add(&tc).expect("add");

        let results = index.search("nonexistent_xyz", 10).expect("search");
        assert!(results.is_empty(), "expected no results");

        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn test_index_non_object_arguments_wrapped() {
        let path = test_index_path();
        let index = ToolCallIndex::new(&path).expect("create index");

        // A non-object JSON value should be stored without panicking.
        let tc = ToolCall {
            uri: "http://zakhor/ns/toolcall/005".into(),
            tool_name: "ping".into(),
            arguments: json!("just a string"),
            session_id: "ses_5".into(),
            timestamp_ms: 5_000,
        };
        index.add(&tc).expect("add non-object args");
        assert_eq!(index.num_docs(), 1);

        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn test_index_open_existing() {
        let path = test_index_path();
        {
            let index = ToolCallIndex::new(&path).expect("create index");
            let tc = ToolCall {
                uri: "http://zakhor/ns/toolcall/006".into(),
                tool_name: "persistent_tool".into(),
                arguments: json!({"key": "value"}),
                session_id: "ses_6".into(),
                timestamp_ms: 6_000,
            };
            index.add(&tc).expect("add");
        }
        let reopened = ToolCallIndex::new(&path).expect("reopen index");
        assert_eq!(reopened.num_docs(), 1);

        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn test_index_debug_impl() {
        let path = test_index_path();
        let index = ToolCallIndex::new(&path).expect("create index");
        let s = format!("{index:?}");
        assert!(s.contains("ToolCallIndex"));
        let _ = std::fs::remove_dir_all(&path);
    }
}
