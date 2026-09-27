//! Integration tests for the GLiNER-RELEX extraction pipeline.
//!
//! These tests load a real ONNX model and tokenizer from disk. They skip
//! automatically when no model can be found, so they are safe to run anywhere.
//!
//! This file used to be gated behind a `gliner-integration` cargo feature that
//! nothing enabled, which meant it was never compiled and its tests never ran —
//! the exact coverage gap that let the extraction breakage in issue #73 go
//! unnoticed. The gate bought nothing, because `ort` and `gline-rs` are
//! unconditional dependencies, so the tests are always built.

use std::path::Path;
use zakhor_model::extraction::{ExtractionConfig, ExtractionPipeline};

/// Locate a usable GLiNER model without requiring configuration or a download.
///
/// `GLINER_MODEL_PATH` wins when set. Otherwise the cache directory is resolved
/// with the same helper the application uses, then scanned with the library's
/// own resolver, so these tests cannot disagree with the app about where the
/// model lives. The old `/models/gliner-relex/...` fallback existed on no
/// machine, so every test here returned early and none had ever run.
fn discover_model() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    if let Ok(p) = std::env::var("GLINER_MODEL_PATH") {
        let model = std::path::PathBuf::from(p);
        let tokenizer = std::path::PathBuf::from(std::env::var("GLINER_TOKENIZER_PATH").ok()?);
        return model.exists().then_some((model, tokenizer));
    }
    let dir = zakhor_common::paths::resolve_gliner_cache_dir(Path::new(""), None);
    let files = zakhor_model::model_setup::find_cached_model(&dir)?;
    Some((files.model_path, files.tokenizer_path))
}

fn load_config() -> Option<ExtractionConfig> {
    let Some((model_path, tokenizer_path)) = discover_model() else {
        eprintln!(
            "skipping: no GLiNER model found. Set GLINER_MODEL_PATH and \
             GLINER_TOKENIZER_PATH, or place the model in the HuggingFace cache."
        );
        return None;
    };

    Some(ExtractionConfig {
        model_path,
        tokenizer_path,
        entity_labels: vec!["person".into(), "organization".into(), "location".into()],
        relation_labels: vec!["works_for".into(), "located_in".into()],
        entity_threshold: 0.5,
        relation_threshold: 0.5,
    })
}

#[tokio::test]
async fn test_extraction_pipeline_extracts_entities_and_relations() {
    let config = match load_config() {
        Some(c) => c,
        None => return,
    };

    let pipeline = ExtractionPipeline::new(config);
    let text = "John works at Google in Mountain View.";

    // The published model cannot be decoded by the pinned binding (#73), so
    // success is not yet reachable. Both outcomes are accepted; what is asserted
    // is that a failure explains itself rather than surfacing the binding's
    // opaque `unexpected logits shape`.
    let entities = match pipeline.extract_entities(text, "").await {
        Ok(found) => {
            assert!(
                !found.is_empty(),
                "a successful extraction must yield entities from: {text}"
            );
            found
        }
        Err(err) => {
            let msg = err.to_string();
            assert!(
                !msg.contains("unexpected logits shape"),
                "the binding's opaque error must not reach the caller: {msg}"
            );
            assert!(
                msg.contains("#73") || msg.contains("unavailable"),
                "failure should be actionable, got: {msg}"
            );
            eprintln!("note: extraction unavailable with this model (#73): {msg}");
            return;
        }
    };

    let relations = pipeline
        .extract_relations(text, &entities, "")
        .await
        .expect("relation extraction should succeed once entities exist");

    assert!(
        !relations.is_empty(),
        "expected at least one relation in: {text}"
    );
}

#[tokio::test]
async fn test_extraction_pipeline_empty_text() {
    let config = match load_config() {
        Some(c) => c,
        None => return,
    };

    let pipeline = ExtractionPipeline::new(config);

    // Empty text has no tokens, so it must be refused before reaching the ONNX
    // graph. Left unchecked it produced a tensor-reshape failure from inside
    // the runtime, which is not something a caller can interpret.
    let err = pipeline
        .extract_entities("   ", "")
        .await
        .expect_err("empty text must be rejected, not sent to the model");
    let msg = err.to_string();
    assert!(
        !msg.contains("Reshape") && !msg.contains("onnxruntime"),
        "a raw runtime error leaked for empty text: {msg}"
    );
    assert!(
        msg.contains("empty text"),
        "error should name the actual problem, got: {msg}"
    );
}
