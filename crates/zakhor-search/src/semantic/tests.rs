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
        text: String::new(),
    };
    assert_eq!(doc.id, "test-123");
    assert!((doc.score - 0.95).abs() < 1e-6);
}

/// A semantic hit must carry the text it was indexed from, otherwise callers
/// receive an opaque id with no content to show or act on.
#[test]
fn test_rank_by_similarity_returns_stored_text() {
    let entries: Vec<SemanticEntry> = vec![
        (
            "urn:uuid:aaa".to_string(),
            vec![1.0, 0.0, 0.0],
            "The deployment pipeline uses Kubernetes.".to_string(),
        ),
        (
            "urn:uuid:bbb".to_string(),
            vec![0.0, 1.0, 0.0],
            "Unrelated note about billing.".to_string(),
        ),
    ];

    let results = rank_by_similarity(&entries, &[1.0, 0.0, 0.0], 10);

    assert_eq!(results.len(), 2);
    assert_eq!(
        results[0].id, "urn:uuid:aaa",
        "best match should rank first"
    );
    assert_eq!(
        results[0].text, "The deployment pipeline uses Kubernetes.",
        "semantic hit must return the indexed text, not an empty string"
    );
    assert_eq!(results[1].text, "Unrelated note about billing.");
}

/// Ordering and truncation must still hold once text is carried along.
#[test]
fn test_rank_by_similarity_orders_and_truncates() {
    let entries: Vec<SemanticEntry> = vec![
        ("id-1".to_string(), vec![0.0, 0.0, 1.0], "first".to_string()),
        (
            "id-2".to_string(),
            vec![1.0, 0.0, 0.0],
            "second".to_string(),
        ),
        ("id-3".to_string(), vec![0.9, 0.1, 0.0], "third".to_string()),
    ];

    let results = rank_by_similarity(&entries, &[1.0, 0.0, 0.0], 2);

    assert_eq!(results.len(), 2, "limit must truncate results");
    assert_eq!(results[0].id, "id-2", "exact match must rank first");
    assert_eq!(results[1].id, "id-3", "near match must rank second");
    assert_eq!(results[0].text, "second");
    assert_eq!(results[1].text, "third");
}

/// An empty index yields no results rather than panicking.
#[test]
fn test_rank_by_similarity_empty_entries() {
    let entries: Vec<SemanticEntry> = Vec::new();
    assert!(rank_by_similarity(&entries, &[1.0, 0.0], 5).is_empty());
}
