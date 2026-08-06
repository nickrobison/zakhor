use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn test_index_path() -> PathBuf {
    let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("zakhor-lexical-test-{n}"));
    // Ensure clean slate
    let _ = std::fs::remove_dir_all(&path);
    path
}

#[test]
fn test_create_and_search() {
    let path = test_index_path();
    let index = LexicalIndex::new(&path).expect("Failed to create index");

    index
        .add("doc-1", "The quick brown fox jumps over the lazy dog", &[])
        .expect("Failed to add document");

    let results = index.search("fox", 10).expect("Failed to search");
    assert!(
        !results.is_empty(),
        "Expected at least one result for 'fox'"
    );
    assert_eq!(results[0].id, "doc-1");
    assert!(
        results[0].score > 0.0,
        "Expected positive BM25 score, got {}",
        results[0].score
    );

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_search_no_match() {
    let path = test_index_path();
    let index = LexicalIndex::new(&path).expect("Failed to create index");

    index
        .add("doc-1", "Hello world", &[])
        .expect("Failed to add document");

    let results = index.search("nonexistent", 10).expect("Failed to search");
    assert!(
        results.is_empty(),
        "Expected no results for nonexistent term"
    );

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_multiple_documents() {
    let path = test_index_path();
    let index = LexicalIndex::new(&path).expect("Failed to create index");

    index
        .add("doc-1", "Rust programming language", &[])
        .unwrap();
    index
        .add("doc-2", "Python programming language", &[])
        .unwrap();
    index
        .add("doc-3", "JavaScript for web development", &[])
        .unwrap();

    let results = index.search("programming", 10).expect("Failed to search");
    assert_eq!(results.len(), 2, "Expected 2 programming docs");
    assert!(results.iter().any(|d| d.id == "doc-1"));
    assert!(results.iter().any(|d| d.id == "doc-2"));

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_open_existing_index() {
    let path = test_index_path();
    {
        let index = LexicalIndex::new(&path).expect("Failed to create index");
        index.add("doc-1", "Persistent data", &[]).unwrap();
    }
    // Reopen the same directory
    let opened = LexicalIndex::new(&path).expect("Failed to open existing index");
    let results = opened.search("Persistent", 10).expect("Failed to search");
    assert!(!results.is_empty(), "Expected results from reopened index");
    assert_eq!(results[0].id, "doc-1");

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_search_limit() {
    let path = test_index_path();
    let index = LexicalIndex::new(&path).expect("Failed to create index");

    for i in 0..10 {
        index
            .add(
                &format!("doc-{i}"),
                &format!("document number {i} with common text"),
                &[],
            )
            .unwrap();
    }

    let results = index.search("common", 3).expect("Failed to search");
    assert_eq!(results.len(), 3, "Expected exactly 3 results with limit=3");

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_debug_impl() {
    let path = test_index_path();
    let index = LexicalIndex::new(&path).expect("Failed to create index");
    let debug_str = format!("{index:?}");
    assert!(
        debug_str.contains("LexicalIndex"),
        "Debug should contain struct name"
    );
    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_with_entity_refs() {
    let path = test_index_path();
    let index = LexicalIndex::new(&path).expect("Failed to create index");

    let refs = vec![
        "http://example.org/e1".into(),
        "http://example.org/e2".into(),
    ];
    index
        .add("doc-1", "Document with entity references", &refs)
        .expect("Failed to add with refs");

    let results = index.search("entity", 10).expect("Failed to search");
    assert!(!results.is_empty(), "Expected result for 'entity'");
    assert_eq!(results[0].id, "doc-1");

    let _ = std::fs::remove_dir_all(&path);
}
