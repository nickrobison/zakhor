use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn test_db_path() -> PathBuf {
    let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("zakhor-sync-test-{n}"));
    let _ = std::fs::remove_dir_all(&path);
    path
}

#[test]
fn test_new_creates_index_dirs() {
    let path = test_db_path();
    let mgr = IndexSyncManager::new(&path, false).expect("Failed to create sync manager");

    assert!(
        path.join("lexical").exists(),
        "lexical index directory should exist at construction"
    );
    assert!(
        !path.join("semantic").exists(),
        "semantic index directory should NOT exist at construction (lazy init)"
    );

    // Debug output should mention the struct name
    let debug = format!("{mgr:?}");
    assert!(debug.contains("IndexSyncManager"), "Debug: {debug}");

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_semantic_dir_not_created_when_disabled() {
    let path = test_db_path();
    let mgr = IndexSyncManager::new(&path, false).expect("Failed to create sync manager");

    // Semantic dir should not exist before first use
    assert!(!path.join("semantic").exists());

    // sync_observation skips semantic when disabled
    mgr.sync_observation("lazy-id", "lazy init test", &[])
        .expect("sync_observation should succeed (lexical only)");

    // Semantic dir should still NOT exist when embedding is disabled
    assert!(
        !path.join("semantic").exists(),
        "semantic index directory should NOT exist when embedding is disabled"
    );

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_sync_observation_adds_to_lexical() {
    let path = test_db_path();
    let mgr = IndexSyncManager::new(&path, false).expect("Failed to create sync manager");

    mgr.sync_observation("test-id", "hello world", &[])
        .expect("Failed to sync observation");

    let results = mgr.lexical.search("hello", 10).expect("Search failed");
    assert!(!results.is_empty(), "Expected results for 'hello'");
    assert_eq!(results[0].id, "test-id");

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_sync_observation_with_entity_refs() {
    let path = test_db_path();
    let mgr = IndexSyncManager::new(&path, false).expect("Failed to create sync manager");

    let refs = vec!["http://example.org/ent1".to_string()];
    mgr.sync_observation("id-1", "entity test", &refs)
        .expect("Failed to sync with refs");

    let results = mgr.lexical.search("entity", 10).expect("Search failed");
    assert!(!results.is_empty(), "Expected results for 'entity'");
    assert_eq!(results[0].id, "id-1");

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_sync_observation_multiple_docs() {
    let path = test_db_path();
    let mgr = IndexSyncManager::new(&path, false).expect("Failed to create sync manager");

    mgr.sync_observation("a", "rust programming", &[]).unwrap();
    mgr.sync_observation("b", "python programming", &[])
        .unwrap();
    mgr.sync_observation("c", "cooking recipes", &[]).unwrap();

    let results = mgr
        .lexical
        .search("programming", 10)
        .expect("Search failed");
    assert_eq!(results.len(), 2, "Expected 2 programming docs");
    assert!(results.iter().any(|d| d.id == "a"));
    assert!(results.iter().any(|d| d.id == "b"));

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_rebuild_all_structure() {
    // rebuild_all requires a real SparqlConnection, so this test
    // verifies that construction and incremental sync work correctly.
    let path = test_db_path();
    let mgr = IndexSyncManager::new(&path, false).expect("Failed to create sync manager");

    mgr.sync_observation("doc-1", "structure test", &[])
        .expect("Failed to sync");

    // Verify the doc was indexed
    let results = mgr.lexical.search("structure", 10).expect("Search failed");
    assert!(!results.is_empty(), "Expected search results");
    assert_eq!(results[0].id, "doc-1");

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_open_existing_indexes() {
    let path = test_db_path();
    let refs = vec!["http://example.org/e1".to_string()];

    // Create and add a document
    {
        let mgr = IndexSyncManager::new(&path, false).expect("First init");
        mgr.sync_observation("persist-id", "persistent data", &refs)
            .expect("First sync");
    }

    // Reopen and search
    {
        let mgr = IndexSyncManager::new(&path, false).expect("Second init");
        let results = mgr.lexical.search("persistent", 10).expect("Search failed");
        assert!(!results.is_empty(), "Expected results from reopened index");
        assert_eq!(results[0].id, "persist-id");
    }

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn test_debug_impl() {
    let path = test_db_path();
    let mgr = IndexSyncManager::new(&path, false).expect("Failed to create");
    let debug = format!("{mgr:?}");
    assert!(
        debug.contains("IndexSyncManager"),
        "Debug should mention IndexSyncManager"
    );
    let _ = std::fs::remove_dir_all(&path);
}
