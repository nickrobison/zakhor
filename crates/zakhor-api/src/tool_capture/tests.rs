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
