//! Integration tests for the GLiNER-RELEX extraction pipeline.
//!
//! These tests load a real ONNX model and tokenizer from disk, so they
//! are feature-gated behind `gliner-integration`.  They will be skipped
//! automatically if the model file does not exist at the configured path.

#![cfg(feature = "gliner-integration")]

use std::path::Path;
use zakhor_model::extraction::{ExtractionConfig, ExtractionPipeline};

const DEFAULT_MODEL_PATH: &str = "/models/gliner-relex/model.onnx";
const DEFAULT_TOKENIZER_PATH: &str = "/models/gliner-relex/tokenizer.json";

fn load_config() -> Option<ExtractionConfig> {
    let model_path = std::env::var("GLINER_MODEL_PATH")
        .unwrap_or_else(|_| DEFAULT_MODEL_PATH.to_string());
    let tokenizer_path = std::env::var("GLINER_TOKENIZER_PATH")
        .unwrap_or_else(|_| DEFAULT_TOKENIZER_PATH.to_string());

    let model_path = Path::new(&model_path);
    if !model_path.exists() {
        eprintln!("skipping: model not found at {}", model_path.display());
        return None;
    }

    Some(ExtractionConfig {
        model_path: model_path.to_path_buf(),
        tokenizer_path: Path::new(&tokenizer_path).to_path_buf(),
        entity_labels: vec![
            "person".into(),
            "organization".into(),
            "location".into(),
        ],
        relation_labels: vec![
            "works_for".into(),
            "located_in".into(),
        ],
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

    let entities = pipeline
        .extract_entities(text)
        .await
        .expect("entity extraction should succeed");

    assert!(
        !entities.is_empty(),
        "expected at least one entity in: {text}"
    );

    let relations = pipeline
        .extract_relations(text, &entities)
        .await
        .expect("relation extraction should succeed");

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

    let entities = pipeline
        .extract_entities("")
        .await
        .expect("entity extraction on empty string should not panic");

    assert!(entities.is_empty(), "expected no entities from empty text");
}
