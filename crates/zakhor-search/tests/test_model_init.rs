//! Standalone test to isolate the ONNX model initialisation hang.
//!
//! Runs the full `SemanticIndex::new(...)` path with a fresh temp DB dir.
//! Uses `HF_HUB_OFFLINE=1` so any network access will fail instantly
//! rather than hanging for minutes, telling us whether the issue is
//! network-related or ONNX-session-related.
//!
//! ```bash
//! HF_HUB_OFFLINE=1 cargo test --test test_model_init -- --nocapture
//! ```

use std::env;
use std::path::PathBuf;
use std::time::Instant;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

/// Path to the pre-seeded cache (we already copied the model here).
const CACHE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

#[test]
fn test_offline_model_init() {
    // Use the actual project db dir so the cache is pre-seeded.
    // (We already migrated the model into ./zakhor-db/semantic/fastembed-cache/)
    let cache_dir = PathBuf::from(CACHE_DIR)
        .join("zakhor-db")
        .join("semantic")
        .join("fastembed-cache");

    assert!(
        cache_dir.exists(),
        "Cache dir {cache_dir:?} does not exist — run `cargo build` first?"
    );
    assert!(
        cache_dir.join("models--Xenova--bge-small-en-v1.5").exists(),
        "BGE model not found in {cache_dir:?}"
    );

    eprintln!(
        "Cache dir: {} (exists: {})",
        cache_dir.display(),
        cache_dir.exists()
    );
    eprintln!("Starting model init (offline mode)…");

    let t0 = Instant::now();

    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::BGESmallENV15)
            .with_cache_dir(cache_dir.clone())
            .with_execution_providers(vec![ort::ep::CPU::default().build()])
            .with_intra_threads(1)
            .with_show_download_progress(true),
    );

    let elapsed = t0.elapsed();
    match model {
        Ok(_) => eprintln!(
            "✓ Model initialised in {}.{:03} s",
            elapsed.as_secs(),
            elapsed.subsec_millis()
        ),
        Err(e) => {
            eprintln!(
                "✗ Model init FAILED after {}.{:03} s: {e:#}",
                elapsed.as_secs(),
                elapsed.subsec_millis()
            );
            panic!("Model init failed: {e:#}");
        }
    }
}
