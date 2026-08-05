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
pub(crate) fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
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
pub(crate) fn cosine_similarity_scalar(a: &[f32], b: &[f32]) -> f64 {
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
