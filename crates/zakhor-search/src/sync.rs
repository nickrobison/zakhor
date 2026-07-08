//! Background index sync manager.
//!
//! Bridges Tracker SPARQL operations with the lexical (Tantivy) and semantic
//! (Fastembed) search indexes. Provides full rebuild and incremental sync.
//!
//! The design avoids GLib event loops (tracker-rs `Notifier` requires GLib) —
//! all sync is triggered explicitly by the MCP `rebuild_indexes` tool or the
//! `--rebuild-indexes` CLI flag.

use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::lexical::LexicalIndex;
use crate::semantic::SemanticIndex;
use zakhor_common::error::{ZakhorError, ZakhorResult};

/// Coordinates lexical + semantic index rebuild and incremental sync.
///
/// Wraps a [`LexicalIndex`] (thread-safe, `&self` methods) and a
/// [`SemanticIndex`] (requires `&mut self`, protected by a `Mutex`).
///
/// **The semantic index is initialized lazily** (via `OnceLock`) — either
/// by a background task spawned at startup or by `get_or_init` on first
/// use. This avoids blocking server startup on model download/load.
pub struct IndexSyncManager {
    pub lexical: LexicalIndex,
    db_path: std::path::PathBuf,
    semantic: OnceLock<Mutex<SemanticIndex>>,
    rebuild_in_progress: Mutex<bool>,
    last_rebuild_finished_at_ms: Mutex<Option<u64>>,
    embedding_enabled: bool,
}

impl std::fmt::Debug for IndexSyncManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexSyncManager").finish()
    }
}

impl IndexSyncManager {
    /// Create or open both indexes at `db_path`.
    ///
    /// The lexical index lives at `<db-path>/lexical/`, the semantic index
    /// lives at `<db-path>/semantic/`. An existing index is opened (not
    /// overwritten) — call [`rebuild_all`](Self::rebuild_all) to re-index
    /// from Tracker.
    pub fn new(db_path: &Path, embedding_enabled: bool) -> ZakhorResult<Self> {
        let lexical = LexicalIndex::new(db_path)?;
        Ok(Self {
            lexical,
            db_path: db_path.to_path_buf(),
            semantic: OnceLock::new(),
            rebuild_in_progress: Mutex::new(false),
            last_rebuild_finished_at_ms: Mutex::new(None),
            embedding_enabled,
        })
    }

    /// Ensure the semantic index is initialized.
    ///
    /// Uses `OnceLock::get_or_init` so that:
    /// - In production: a background task usually `set()`s the cell first, so
    ///   this returns instantly.
    /// - Otherwise (tests, `--rebuild-indexes`) this falls back to a synchronous
    ///   init.
    fn ensure_semantic(&self) -> ZakhorResult<std::sync::MutexGuard<'_, SemanticIndex>> {
        if !self.embedding_enabled {
            return Err(
                ZakhorError::Internal("Embeddings are disabled via config".to_string()).into(),
            );
        }
        let cell = self.semantic.get_or_init(|| {
            let span = tracing::info_span!(
                "semantic_lazy_init",
                model = "BGESmallENV15",
                cache = %self.db_path.join("semantic").join("fastembed-cache").display(),
            );
            let _enter = span.enter();
            let sem = SemanticIndex::new(&self.db_path)
                .expect("SemanticIndex init failed — critical model missing");
            Mutex::new(sem)
        });
        cell.lock()
            .map_err(|e| ZakhorError::Internal(format!("Semantic lock poisoned: {e}")))
            .map_err(Into::into)
    }

    /// Full rebuild: delete all docs from both indexes, re-query every
    /// `nie:InformationElement` from Tracker, re-index both Tantivy and
    /// Fastembed, then snapshot the semantic index to disk.
    ///
    /// Updates `rebuild_in_progress` and `last_rebuild_finished_at_ms`
    /// automatically. Returns immediately with `Err` if a rebuild is already
    /// in progress.
    pub fn rebuild_all(&self, conn: &tracker::SparqlConnection) -> ZakhorResult<()> {
        let mut guard = self
            .rebuild_in_progress
            .lock()
            .map_err(|e| ZakhorError::Internal(format!("Rebuild progress lock poisoned: {e}")))?;
        if *guard {
            return Err(ZakhorError::Internal("Rebuild already in progress".to_string()).into());
        }
        *guard = true;

        let result = (|| -> ZakhorResult<()> {
            // Lexical rebuild — uses &self, no mutex
            self.lexical.rebuild_from_tracker(conn)?;

            // Semantic rebuild — lazy-init the model on first call
            if self.embedding_enabled {
                let mut guard = self.ensure_semantic()?;
                guard
                    .rebuild_from_tracker(conn)
                    .map_err(|e| ZakhorError::Internal(format!("Semantic rebuild failed: {e}")))?;
                guard
                    .snapshot()
                    .map_err(|e| ZakhorError::Internal(format!("Semantic snapshot failed: {e}")))?;
            }

            // Record the finish timestamp
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            if let Ok(mut ts) = self.last_rebuild_finished_at_ms.lock() {
                *ts = Some(now);
            }

            Ok(())
        })();

        *guard = false;
        drop(guard);

        result
    }

    /// Number of vectors in the semantic index (0 if not yet initialized or disabled).
    pub fn semantic_len(&self) -> usize {
        if !self.embedding_enabled {
            return 0;
        }
        self.semantic
            .get()
            .and_then(|m| m.lock().ok().map(|g| g.len()))
            .unwrap_or(0)
    }

    /// Search the semantic index (lazy-init safe, returns empty vec if not initialized or disabled).
    pub fn semantic_search(&self, query: &str, limit: usize) -> Vec<crate::ScoredDoc> {
        if !self.embedding_enabled {
            return Vec::new();
        }
        match self.semantic.get() {
            Some(m) => match m.lock() {
                Ok(mut guard) => guard.search(query, limit),
                Err(_) => Vec::new(),
            },
            None => Vec::new(),
        }
    }

    /// Returns `true` if a full-index rebuild is currently running.
    pub fn is_rebuild_in_progress(&self) -> bool {
        self.rebuild_in_progress.lock().map(|g| *g).unwrap_or(false)
    }

    /// Returns the UNIX-millis timestamp of the last completed rebuild, or
    /// `None` if no rebuild has finished yet.
    pub fn last_rebuild_ms(&self) -> Option<u64> {
        self.last_rebuild_finished_at_ms
            .lock()
            .ok()
            .and_then(|g| *g)
    }

    /// Incremental sync: add or update a single observation in both indexes.
    ///
    /// `entity_refs` is a comma-separated list of entity URIs stored in the
    /// lexical index for future entity-aware search.
    pub fn sync_observation(
        &self,
        id: &str,
        text: &str,
        entity_refs: &[String],
    ) -> ZakhorResult<()> {
        // Lexical add — &self, no mutex
        self.lexical.add(id, text, entity_refs)?;

        // Semantic add — lazy-init the model on first call
        if self.embedding_enabled {
            let mut guard = self.ensure_semantic()?;
            guard
                .add(id, text)
                .map_err(|e| ZakhorError::Internal(format!("Semantic add failed: {e}")))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
}
