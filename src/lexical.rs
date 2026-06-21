//! Tantivy-based lexical full-text search index.
//!
//! This is a **derived projection** of GNOME Tracker data — Tracker is the
//! source of truth; this index provides BM25 keyword search over memory text.
//! The index lives at `<db-path>/lexical/` and is rebuilt from Tracker on
//! demand via [`LexicalIndex::rebuild_from_tracker`].

use std::path::{Path, PathBuf};

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, IndexWriter, TantivyDocument, doc};
use tracker::prelude::SparqlCursorExtManual;

use crate::error::{ZakhorError, ZakhorResult};
use crate::semantic::ScoredDoc;

/// Lexical BM25 search index backed by Tantivy.
///
/// Wraps a Tantivy index with three fields:
/// - `id`: STRING + STORED (not tokenized, retrievable)
/// - `text`: TEXT + STORED (tokenized for BM25, retrievable)
/// - `entity_refs`: STRING + STORED (comma-separated entity URIs)
pub struct LexicalIndex {
    index: Index,
    index_path: PathBuf,
    id_field: Field,
    text_field: Field,
    entity_refs_field: Field,
}

impl std::fmt::Debug for LexicalIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LexicalIndex")
            .field("index_path", &self.index_path)
            .finish()
    }
}

impl LexicalIndex {
    /// Build the Tantivy schema for the lexical index.
    fn build_schema() -> Schema {
        let mut builder = Schema::builder();
        builder.add_text_field("id", STRING | STORED);
        builder.add_text_field("text", TEXT | STORED);
        builder.add_text_field("entity_refs", STRING | STORED);
        builder.build()
    }

    /// Create or open a Tantivy index at `<db-path>/lexical/`.
    ///
    /// If the directory exists, the existing index is opened and its schema
    /// is validated. If it does not exist, a new index is created.
    pub fn new(db_path: &Path) -> ZakhorResult<Self> {
        let index_path = db_path.join("lexical");

        let index: Index = if index_path.exists() {
            Index::open_in_dir(&index_path)
                .map_err(|e| ZakhorError::Internal(format!("Failed to open Tantivy index: {e}")))?
        } else {
            std::fs::create_dir_all(&index_path)
                .map_err(|e| ZakhorError::Internal(format!("Failed to create index dir: {e}")))?;
            let schema = Self::build_schema();
            Index::create_in_dir(&index_path, schema).map_err(|e| {
                ZakhorError::Internal(format!("Failed to create Tantivy index: {e}"))
            })?
        };

        let schema = index.schema();
        let id_field = schema
            .get_field("id")
            .map_err(|e| ZakhorError::Internal(format!("Schema missing id field: {e}")))?;
        let text_field = schema
            .get_field("text")
            .map_err(|e| ZakhorError::Internal(format!("Schema missing text field: {e}")))?;
        let entity_refs_field = schema
            .get_field("entity_refs")
            .map_err(|e| ZakhorError::Internal(format!("Schema missing entity_refs field: {e}")))?;

        Ok(Self {
            index,
            index_path,
            id_field,
            text_field,
            entity_refs_field,
        })
    }

    /// Add a single document to the index.
    ///
    /// The document is immediately committed so it becomes visible to
    /// subsequent searches. `entity_refs` is stored as a comma-separated
    /// string of entity URIs.
    pub fn add(&self, id: &str, text: &str, entity_refs: &[String]) -> ZakhorResult<()> {
        let mut writer: IndexWriter = self
            .index
            .writer(50_000_000)
            .map_err(|e| ZakhorError::Internal(format!("Failed to create writer: {e}")))?;

        let refs = entity_refs.join(",");
        let _ = writer.add_document(doc!(
            self.id_field => id,
            self.text_field => text,
            self.entity_refs_field => refs,
        ));
        writer
            .commit()
            .map_err(|e| ZakhorError::Internal(format!("Failed to commit: {e}")))?;

        Ok(())
    }

    /// Search the index with BM25 ranking.
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

        let query_parser = QueryParser::for_index(&self.index, vec![self.text_field]);
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
    ///
    /// Returns 0 if the reader cannot be opened (e.g. before any documents
    /// have been indexed).
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

    /// Rebuild the entire index from Tracker SPARQL data.
    ///
    /// This deletes all existing documents and re-indexes every
    /// `nie:InformationElement` from the connected Tracker store.
    /// `entity_refs` is left as an empty string — that field is reserved
    /// for future entity-linking enrichment.
    pub fn rebuild_from_tracker(&self, conn: &tracker::SparqlConnection) -> ZakhorResult<()> {
        let mut writer: IndexWriter = self
            .index
            .writer(50_000_000)
            .map_err(|e| ZakhorError::Internal(format!("Failed to create writer: {e}")))?;

        writer
            .delete_all_documents()
            .map_err(|e| ZakhorError::Internal(format!("Failed to clear index: {e}")))?;

        let sparql = "\
            PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>\n\
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
            SELECT ?identifier ?text WHERE {\n\
                ?id rdf:type nie:InformationElement ;\n\
                    nie:identifier ?identifier ;\n\
                    nie:plainTextContent ?text .\n\
            }";

        let cursor = conn
            .query(sparql, None::<&gio::Cancellable>)
            .map_err(|e| ZakhorError::Database(format!("SPARQL query failed: {e}")))?;

        while cursor
            .next(None::<&gio::Cancellable>)
            .map_err(|e| ZakhorError::Database(format!("Cursor iteration failed: {e}")))?
        {
            let id = cursor
                .string(0)
                .ok_or_else(|| ZakhorError::Internal("Missing identifier in SPARQL result".into()))?
                .to_string();
            let text = cursor
                .string(1)
                .ok_or_else(|| ZakhorError::Internal("Missing text in SPARQL result".into()))?
                .to_string();

            let _ = writer.add_document(doc!(
                self.id_field => id,
                self.text_field => text,
                self.entity_refs_field => "",
            ));
        }

        writer
            .commit()
            .map_err(|e| ZakhorError::Internal(format!("Failed to commit rebuild: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
}
