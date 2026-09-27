use super::cache::migrate_cache_from_legacy_location;
use super::simd::cosine_similarity;
use crate::semantic::ScoredDoc;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::path::{Path, PathBuf};
use tracker::prelude::SparqlCursorExtManual;

/// A stored document: its id, its embedding, and the text it was derived from.
///
/// The text is retained so that [`ScoredDoc`] results can carry their content.
/// Without it a semantic hit can only report an opaque id.
pub(crate) type SemanticEntry = (String, Vec<f32>, String);

/// Rank `entries` against `query_vec` by cosine similarity, best first.
///
/// Pure function: no model required, so it is directly unit-testable.
pub(crate) fn rank_by_similarity(
    entries: &[SemanticEntry],
    query_vec: &[f32],
    limit: usize,
) -> Vec<ScoredDoc> {
    let mut scored: Vec<ScoredDoc> = entries
        .iter()
        .map(|(id, vec, text)| ScoredDoc {
            id: id.clone(),
            score: cosine_similarity(query_vec, vec),
            text: text.clone(),
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(limit);
    scored
}

/// In-memory semantic vector index using `fastembed` for local CPU embeddings.
///
/// Uses `BAAI/bge-small-en-v1.5` (384-dim) by default. Snapshots are persisted
/// to `<db-path>/semantic/vectors.bin` via bincode. This index is a *derived
/// projection* — the Tracker SPARQL store remains the source of truth.
pub struct SemanticIndex {
    model: TextEmbedding,
    vectors: Vec<SemanticEntry>,
    snapshot_path: PathBuf,
}

impl SemanticIndex {
    /// Create a new index at `db_path`, loading the model and any existing snapshot.
    ///
    /// The snapshot directory `<db-path>/semantic/` is created if missing.
    /// The embedding model binary is cached under `<db-path>/semantic/fastembed-cache/`
    /// so that re-initialisation does not re-download the ~70 MB model file.
    pub fn new(db_path: &Path) -> Result<Self, String> {
        let snapshot_path = db_path.join("semantic").join("vectors.bin");
        let cache_dir = db_path.join("semantic").join("fastembed-cache");
        let semantic_dir = snapshot_path
            .parent()
            .expect("snapshot path must have parent directory");
        std::fs::create_dir_all(semantic_dir)
            .map_err(|e| format!("Failed to create semantic dir: {}", e))?;

        // If the new cache is empty but the old default fastembed cache has the
        // model, migrate it so we don't re-download the ~133 MB ONNX model.
        migrate_cache_from_legacy_location(&cache_dir);

        tracing::info!("Initialising ONNX inference session (this may take ~30 s on first run)");
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15)
                .with_cache_dir(cache_dir)
                .with_execution_providers(vec![ort::ep::CPU::default().build()])
                .with_intra_threads(1)
                .with_show_download_progress(true),
        )
        .map_err(|e| format!("Failed to init embedding model: {}", e))?;
        tracing::info!("ONNX inference session ready");

        let mut index = Self {
            model,
            vectors: Vec::new(),
            snapshot_path,
        };

        if index.snapshot_path.exists() {
            // A snapshot written by an older build may use an incompatible
            // layout. This index is a derived projection — Tracker remains the
            // source of truth — so start empty and let `rebuild_indexes`
            // repopulate rather than refusing to start.
            if let Err(e) = index.load() {
                tracing::warn!(
                    path = %index.snapshot_path.display(),
                    error = %e,
                    "Could not load semantic snapshot; starting empty. Run rebuild_indexes to repopulate."
                );
            }
        }

        Ok(index)
    }

    /// Embed `text` and store the vector under `id`.
    ///
    /// Calling this with the same `id` appends a duplicate entry;
    /// deduplication is the caller's responsibility.
    pub fn add(&mut self, id: &str, text: &str) -> Result<(), String> {
        let embeddings = self
            .model
            .embed(vec![text.to_string()], None)
            .map_err(|e| format!("Embedding failed: {}", e))?;
        let embedding = embeddings
            .into_iter()
            .next()
            .expect("embedding should produce exactly one vector");
        self.vectors
            .push((id.to_string(), embedding, text.to_string()));
        Ok(())
    }

    /// Search the index by cosine similarity.
    ///
    /// Returns up to `limit` results sorted by descending score.
    /// Returns an empty vec when the index is empty.
    pub fn search(&mut self, query: &str, limit: usize) -> Vec<ScoredDoc> {
        if self.vectors.is_empty() {
            return Vec::new();
        }

        let query_vec = match self.model.embed(vec![query.to_string()], None) {
            Ok(mut embeddings) => embeddings.swap_remove(0),
            Err(_) => return Vec::new(),
        };

        rank_by_similarity(&self.vectors, &query_vec, limit)
    }

    /// Number of vectors currently in the index.
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Returns `true` when the index has no vectors.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Persist the vector index to disk via bincode.
    pub fn snapshot(&self) -> Result<(), String> {
        let data =
            bincode::serialize(&self.vectors).map_err(|e| format!("Serialize failed: {}", e))?;
        std::fs::write(&self.snapshot_path, data)
            .map_err(|e| format!("Write snapshot failed: {}", e))
    }

    /// Restore the vector index from a bincode snapshot on disk.
    pub fn load(&mut self) -> Result<(), String> {
        let data = std::fs::read(&self.snapshot_path)
            .map_err(|e| format!("Read snapshot failed: {}", e))?;
        self.vectors =
            bincode::deserialize(&data).map_err(|e| format!("Deserialize failed: {}", e))?;
        Ok(())
    }

    /// Rebuild the entire index from the Tracker SPARQL store.
    ///
    /// Clears all existing vectors, queries every stored memory
    /// (identifier + text content), and re-embeds each one.
    pub fn rebuild_from_tracker(&mut self, conn: &tracker::SparqlConnection) -> Result<(), String> {
        self.vectors.clear();

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
            .map_err(|e| format!("SPARQL query failed: {}", e))?;

        while cursor
            .next(None::<&gio::Cancellable>)
            .map_err(|e| format!("Cursor iteration failed: {}", e))?
        {
            let id = cursor
                .string(0)
                .ok_or_else(|| "Missing identifier".to_string())?
                .to_string();
            let text = cursor
                .string(1)
                .ok_or_else(|| "Missing text content".to_string())?
                .to_string();
            self.add(&id, &text)?;
        }

        Ok(())
    }
}
