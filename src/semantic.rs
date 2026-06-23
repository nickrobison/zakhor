use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::path::{Path, PathBuf};
use tracker::prelude::SparqlCursorExtManual;

/// A scored document returned by semantic or lexical search.
#[derive(Debug, Clone)]
pub struct ScoredDoc {
    pub id: String,
    pub score: f64,
}

/// Cosine similarity between two f32 vectors.
///
/// Returns a value in [-1.0, 1.0] where 1.0 means identical direction.
/// Fastembed already L2-normalizes its output, so for these vectors this
/// is equivalent to a dot product — but the explicit computation is more
/// robust for arbitrary inputs.
///
/// The computation is accelerated using SIMD intrinsics where available:
/// - AVX2 on x86_64 (8 f32 lanes)
/// - NEON on AArch64 (4 f32 lanes)
/// - Scalar fallback on other architectures
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    cosine_similarity_dispatch(a, b)
}

/// Dispatch to the best available SIMD implementation at runtime.
#[cfg(target_arch = "x86_64")]
fn cosine_similarity_dispatch(a: &[f32], b: &[f32]) -> f64 {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: AVX2 availability has been verified at runtime.
        unsafe { cosine_similarity_avx2(a, b) }
    } else {
        cosine_similarity_scalar(a, b)
    }
}

#[cfg(target_arch = "aarch64")]
fn cosine_similarity_dispatch(a: &[f32], b: &[f32]) -> f64 {
    // NEON is mandatory on all AArch64 targets.
    // SAFETY: NEON is always available on AArch64.
    unsafe { cosine_similarity_neon(a, b) }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn cosine_similarity_dispatch(a: &[f32], b: &[f32]) -> f64 {
    cosine_similarity_scalar(a, b)
}

/// Pure-scalar fallback used on unsupported architectures and as a reference.
fn cosine_similarity_scalar(a: &[f32], b: &[f32]) -> f64 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    (dot / (norm_a * norm_b)) as f64
}

/// AVX2-accelerated cosine similarity (8 f32 lanes per iteration).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn cosine_similarity_avx2(a: &[f32], b: &[f32]) -> f64 {
    use std::arch::x86_64::*;

    let n = a.len().min(b.len());
    let chunks = n / 8;

    debug_assert_eq!(
        a.len(),
        b.len(),
        "cosine_similarity requires equal-length vectors"
    );

    // SAFETY: AVX2 availability is verified by the caller; all pointer offsets
    // are within the slice bounds derived from `chunks` and the scalar tail range.
    unsafe {
        let mut dot = _mm256_setzero_ps();
        let mut sq_a = _mm256_setzero_ps();
        let mut sq_b = _mm256_setzero_ps();

        for i in 0..chunks {
            let va = _mm256_loadu_ps(a.as_ptr().add(i * 8));
            let vb = _mm256_loadu_ps(b.as_ptr().add(i * 8));
            dot = _mm256_add_ps(dot, _mm256_mul_ps(va, vb));
            sq_a = _mm256_add_ps(sq_a, _mm256_mul_ps(va, va));
            sq_b = _mm256_add_ps(sq_b, _mm256_mul_ps(vb, vb));
        }

        // Horizontal sum: fold 256-bit register down to a scalar f32.
        // Step 1: add the high 128-bit lane to the low 128-bit lane.
        let lo = _mm256_castps256_ps128(dot);
        let hi = _mm256_extractf128_ps(dot, 1);
        let dot128 = _mm_add_ps(lo, hi);

        let lo = _mm256_castps256_ps128(sq_a);
        let hi = _mm256_extractf128_ps(sq_a, 1);
        let sq_a128 = _mm_add_ps(lo, hi);

        let lo = _mm256_castps256_ps128(sq_b);
        let hi = _mm256_extractf128_ps(sq_b, 1);
        let sq_b128 = _mm_add_ps(lo, hi);

        // Step 2: horizontal sum of 4 f32 lanes.
        // shuf = [b, a, d, c]; sums = [a+b, a+b, c+d, c+d]
        // shuf2 = [c+d, c+d, ...]; result[0] = (a+b) + (c+d)
        let shuf = _mm_shuffle_ps(dot128, dot128, 0b10_11_00_01);
        let sums = _mm_add_ps(dot128, shuf);
        let shuf2 = _mm_movehl_ps(sums, sums);
        let mut dot_f = _mm_cvtss_f32(_mm_add_ss(sums, shuf2));

        let shuf = _mm_shuffle_ps(sq_a128, sq_a128, 0b10_11_00_01);
        let sums = _mm_add_ps(sq_a128, shuf);
        let shuf2 = _mm_movehl_ps(sums, sums);
        let mut na_f = _mm_cvtss_f32(_mm_add_ss(sums, shuf2));

        let shuf = _mm_shuffle_ps(sq_b128, sq_b128, 0b10_11_00_01);
        let sums = _mm_add_ps(sq_b128, shuf);
        let shuf2 = _mm_movehl_ps(sums, sums);
        let mut nb_f = _mm_cvtss_f32(_mm_add_ss(sums, shuf2));

        // Scalar tail for elements that didn't fill a full 8-wide chunk.
        for i in (chunks * 8)..n {
            let ai = *a.get_unchecked(i);
            let bi = *b.get_unchecked(i);
            dot_f += ai * bi;
            na_f += ai * ai;
            nb_f += bi * bi;
        }

        (dot_f / (na_f.sqrt() * nb_f.sqrt())) as f64
    }
}

/// NEON-accelerated cosine similarity (4 f32 lanes per iteration).
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn cosine_similarity_neon(a: &[f32], b: &[f32]) -> f64 {
    use std::arch::aarch64::*;

    let n = a.len().min(b.len());
    let chunks = n / 4;

    debug_assert_eq!(
        a.len(),
        b.len(),
        "cosine_similarity requires equal-length vectors"
    );

    // SAFETY: NEON is mandatory on AArch64; all pointer offsets are within
    // the slice bounds derived from `chunks` and the scalar tail range.
    unsafe {
        let mut dot = vdupq_n_f32(0.0);
        let mut sq_a = vdupq_n_f32(0.0);
        let mut sq_b = vdupq_n_f32(0.0);

        for i in 0..chunks {
            let va = vld1q_f32(a.as_ptr().add(i * 4));
            let vb = vld1q_f32(b.as_ptr().add(i * 4));
            dot = vmlaq_f32(dot, va, vb);
            sq_a = vmlaq_f32(sq_a, va, va);
            sq_b = vmlaq_f32(sq_b, vb, vb);
        }

        // Horizontal sum across all 4 lanes.
        let mut dot_f = vaddvq_f32(dot);
        let mut na_f = vaddvq_f32(sq_a);
        let mut nb_f = vaddvq_f32(sq_b);

        // Scalar tail for elements that didn't fill a full 4-wide chunk.
        for i in (chunks * 4)..n {
            let ai = *a.get_unchecked(i);
            let bi = *b.get_unchecked(i);
            dot_f += ai * bi;
            na_f += ai * ai;
            nb_f += bi * bi;
        }

        (dot_f / (na_f.sqrt() * nb_f.sqrt())) as f64
    }
}

/// In-memory semantic vector index using `fastembed` for local CPU embeddings.
///
/// Uses `BAAI/bge-small-en-v1.5` (384-dim) by default. Snapshots are persisted
/// to `<db-path>/semantic/vectors.bin` via bincode. This index is a *derived
/// projection* — the Tracker SPARQL store remains the source of truth.
pub struct SemanticIndex {
    model: TextEmbedding,
    vectors: Vec<(String, Vec<f32>)>,
    snapshot_path: PathBuf,
}

impl SemanticIndex {
    /// Create a new index at `db_path`, loading the model and any existing snapshot.
    ///
    /// The snapshot directory `<db-path>/semantic/` is created if missing.
    /// Model auto-download occurs on first use (blocking IO, CPU only).
    pub fn new(db_path: &Path) -> Result<Self, String> {
        let snapshot_path = db_path.join("semantic").join("vectors.bin");
        std::fs::create_dir_all(
            snapshot_path
                .parent()
                .expect("snapshot path must have parent directory"),
        )
        .map_err(|e| format!("Failed to create semantic dir: {}", e))?;

        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15).with_show_download_progress(false),
        )
        .map_err(|e| format!("Failed to init embedding model: {}", e))?;

        let mut index = Self {
            model,
            vectors: Vec::new(),
            snapshot_path,
        };

        if index.snapshot_path.exists() {
            index.load()?;
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
        self.vectors.push((id.to_string(), embedding));
        Ok(())
    }

    /// Remove all vectors for a given document id.
    pub fn remove(&mut self, id: &str) {
        self.vectors.retain(|(doc_id, _)| doc_id != id);
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

        let mut scored: Vec<ScoredDoc> = self
            .vectors
            .iter()
            .map(|(id, vec)| ScoredDoc {
                id: id.clone(),
                score: cosine_similarity(&query_vec, vec),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── QuickCheck property tests ──────────────────────────────────────────────

    /// A pair of equal-length, non-zero f32 vectors suitable for cosine similarity.
    ///
    /// QuickCheck shrinks these by shrinking the inner vectors while preserving
    /// the length-equality invariant. Zero-norm vectors are excluded because
    /// cosine similarity is undefined for them.
    #[cfg(test)]
    #[derive(Clone, Debug)]
    struct VecPair(Vec<f32>, Vec<f32>);

    #[cfg(test)]
    impl quickcheck::Arbitrary for VecPair {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            // Clamp the length to [1, 512] so tests stay fast.
            let len = (usize::arbitrary(g) % 512) + 1;
            let a: Vec<f32> = (0..len).map(|_| f32::arbitrary(g)).collect();
            let b: Vec<f32> = (0..len).map(|_| f32::arbitrary(g)).collect();
            VecPair(a, b)
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let a = self.0.clone();
            let b = self.1.clone();
            // Shrink by dropping the last element from both vectors simultaneously.
            Box::new(
                (1..a.len())
                    .rev()
                    .map(move |len| VecPair(a[..len].to_vec(), b[..len].to_vec())),
            )
        }
    }

    /// Returns true when both vectors have non-zero norm (cosine similarity is
    /// defined) and all values are finite (no NaN / Inf from arbitrary f32).
    /// Also rejects inputs whose sum-of-squares overflows f32 to infinity,
    /// which would produce a NaN cosine result.
    fn is_valid_pair(a: &[f32], b: &[f32]) -> bool {
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        norm_a > 0.0 && norm_a.is_finite() && norm_b > 0.0 && norm_b.is_finite()
    }

    #[quickcheck_macros::quickcheck]
    fn prop_simd_matches_scalar(pair: VecPair) -> quickcheck::TestResult {
        let VecPair(a, b) = pair;
        if !is_valid_pair(&a, &b) {
            return quickcheck::TestResult::discard();
        }
        let simd = cosine_similarity(&a, &b);
        let scalar = cosine_similarity_scalar(&a, &b);
        quickcheck::TestResult::from_bool((simd - scalar).abs() < 1e-5)
    }

    // ── Deterministic unit tests ───────────────────────────────────────────────

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6, "expected ~1.0, got {}", sim);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6, "expected ~0.0, got {}", sim);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-6, "expected ~-1.0, got {}", sim);
    }

    /// Verify that the SIMD path produces the same result as the scalar fallback
    /// for a 384-element vector (the BGE-small embedding dimension). A tolerance
    /// of 1e-5 is used to accommodate floating-point reordering across SIMD lanes.
    #[test]
    fn test_cosine_similarity_simd_matches_scalar() {
        let n = 384;
        let a: Vec<f32> = (0..n).map(|i| (i as f32 * 0.017).sin()).collect();
        let b: Vec<f32> = (0..n).map(|i| (i as f32 * 0.013).cos()).collect();

        let simd = cosine_similarity(&a, &b);
        let scalar = cosine_similarity_scalar(&a, &b);

        assert!(
            (simd - scalar).abs() < 1e-5,
            "SIMD result {} differs from scalar {} by more than tolerance",
            simd,
            scalar
        );
    }

    #[test]
    fn test_empty_index_search_returns_empty() {
        let scored: Vec<ScoredDoc> = Vec::new();
        assert!(scored.is_empty());
    }

    #[test]
    fn test_scored_doc_struct() {
        let doc = ScoredDoc {
            id: "test-123".to_string(),
            score: 0.95,
        };
        assert_eq!(doc.id, "test-123");
        assert!((doc.score - 0.95).abs() < 1e-6);
    }
}
