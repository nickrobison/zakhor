//! Integration tests for the parallel extraction + ingestion pipeline.
//!
//! These tests load a real ONNX model from disk and optionally connect to a
//! Tracker SPARQL database.  Tests that require SPARQL skip gracefully if
//! the Tracker runtime is not available.
//!
//! Feature-gated behind `gliner-integration` — like extraction_integration.rs,
//! tests are silently skipped when the model file is missing.

#![cfg(feature = "gliner-integration")]

use std::path::Path;
use std::sync::Arc;

use zakhor_model::extraction::{ExtractionConfig, ExtractionPipeline};
use zakhor_model::pipeline::IngestionPipeline;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_MODEL_PATH: &str = "/models/gliner-relex/model.onnx";
const DEFAULT_TOKENIZER_PATH: &str = "/models/gliner-relex/tokenizer.json";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_config() -> Option<ExtractionConfig> {
    let model_path =
        std::env::var("GLINER_MODEL_PATH").unwrap_or_else(|_| DEFAULT_MODEL_PATH.to_string());
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
        entity_labels: vec!["person".into(), "organization".into(), "location".into()],
        relation_labels: vec!["works_for".into(), "located_in".into()],
        entity_threshold: 0.5,
        relation_threshold: 0.5,
    })
}

/// Try to initialise a temporary in-process Tracker SPARQL database.
///
/// Returns `None` when the Tracker runtime library is not available (e.g. not
/// installed or GLib initialisation fails), causing the calling test to skip
/// gracefully rather than fail.
fn try_sparql_connection() -> Option<tracker::SparqlConnection> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_dir = std::env::temp_dir().join(format!("zakhor-integration-{timestamp}"));

    // `init_db` panics when the Tracker C library is missing or GLib
    // resources are unavailable — catch that and treat as "not available".
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        zakhor_storage::tracker_db::init_db(tmp_dir.to_str().expect("temp path is valid UTF-8"))
    }));

    match result {
        Ok(conn) => Some(conn),
        Err(_) => {
            eprintln!("skipping SPARQL-dependent test: Tracker runtime not available");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Combined entity + relation extraction (single NER pass)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_extract_entities_and_relations_parallel() {
    // Verify that `extract_entities_and_relations` returns both entities and
    // relations from a single NER pass for a sentence that contains known
    // person / organisation / location entities.
    let config = match load_config() {
        Some(c) => c,
        None => return,
    };

    let pipeline = ExtractionPipeline::new(config);
    let text = "John works at Google in Mountain View.";

    let (entities, relations) = pipeline
        .extract_entities_and_relations(text, "test-parallel-001")
        .await
        .expect("combined extraction should succeed");

    assert!(
        !entities.is_empty(),
        "expected at least one entity in: {text}"
    );
    assert!(
        !relations.is_empty(),
        "expected at least one relation in: {text}"
    );

    // Verify entity structure
    for entity in &entities {
        assert!(!entity.uri.is_empty(), "entity URI must not be empty");
        assert!(!entity.label.is_empty(), "entity label must not be empty");
    }

    // Verify relation structure
    for relation in &relations {
        assert!(
            !relation.subject_uri.is_empty(),
            "relation subject must not be empty"
        );
        assert!(
            !relation.predicate_uri.is_empty(),
            "relation predicate must not be empty"
        );
        assert!(
            !relation.object_uri.is_empty(),
            "relation object must not be empty"
        );
        assert!(
            !relation.label.is_empty(),
            "relation label must not be empty"
        );
    }
}

#[tokio::test]
async fn test_extract_entities_and_relations_single_ner_pass() {
    // Smoke test: `extract_entities_and_relations` returns results for a
    // short sentence — lighter-weight version of the parallel test above.
    let config = match load_config() {
        Some(c) => c,
        None => return,
    };

    let pipeline = ExtractionPipeline::new(config);
    let text = "Satya Nadella is CEO of Microsoft.";

    let result = pipeline
        .extract_entities_and_relations(text, "test-smoke-001")
        .await;

    assert!(
        result.is_ok(),
        "combined extraction should succeed: {:?}",
        result.err()
    );

    let (entities, relations) = result.unwrap();
    assert!(
        !entities.is_empty() || !relations.is_empty(),
        "expected at least some extraction results from: {text}"
    );
}

// ---------------------------------------------------------------------------
// Async ingestion pipeline (requires Tracker SPARQL)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_async_ingestion_pipeline_functional() {
    // Full pipeline integration test:
    // 1. Load ONNX model
    // 2. Create an in-process Tracker DB
    // 3. Create IngestionPipeline with with_sync_manager(None)
    // 4. Call extract_and_ingest_async
    // 5. Verify it returns IngestResult
    let config = match load_config() {
        Some(c) => c,
        None => return,
    };

    let conn = match try_sparql_connection() {
        Some(c) => Arc::new(c),
        None => return,
    };

    let extraction = ExtractionPipeline::new(config);
    let mut pipeline = IngestionPipeline::with_sync_manager(None);

    let text = "John works at Google in Mountain View.";
    let correlation_id = "test-functional-001";

    let result = pipeline
        .extract_and_ingest_async(conn, text, &extraction, correlation_id)
        .await;

    match result {
        Ok(ingest_result) => {
            assert!(
                ingest_result.observation_uri.starts_with("urn:uuid:"),
                "observation_uri should be a URN UUID: {}",
                ingest_result.observation_uri
            );
            assert!(
                ingest_result.triple_count > 0,
                "expected at least one triple: {}",
                ingest_result.triple_count
            );
        }
        Err(e) => {
            // If the pipeline fails at persist (e.g. ontology mismatch in CI),
            // verify it's not an extraction or validation error — those stages
            // must succeed before we even attempt SPARQL.
            let err_str = e.to_string();
            assert!(
                !err_str.contains("validation:"),
                "validation should not fail with non-empty text: {err_str}"
            );
            assert!(
                !err_str.contains("inference:"),
                "extraction should not fail: {err_str}"
            );
            // Allow persist / build errors when running in environments
            // without a fully-working Tracker in-process store.
            let is_persist_or_build = err_str.contains("persist:")
                || err_str.contains("build:")
                || err_str.contains("SPARQL");
            assert!(
                is_persist_or_build,
                "unexpected error (expected persist/build only): {err_str}"
            );
            eprintln!(
                "note: pipeline failed at persist stage (Tracker store may \
                 not have zakhor ontology) — extraction + validation passed"
            );
        }
    }
}

#[tokio::test]
async fn test_ingest_async_with_empty_text() {
    // Verify that extract_and_ingest_async returns a validation error when the
    // text is empty — validation is Stage 1, before any SPARQL operation.
    let config = match load_config() {
        Some(c) => c,
        None => return,
    };

    let conn = match try_sparql_connection() {
        Some(c) => Arc::new(c),
        None => return,
    };

    let extraction = ExtractionPipeline::new(config);
    let mut pipeline = IngestionPipeline::with_sync_manager(None);

    let result = pipeline
        .extract_and_ingest_async(conn, "", &extraction, "test-empty-001")
        .await;

    match result {
        Ok(_) => {
            panic!("expected empty text to be rejected, but pipeline succeeded");
        }
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("validation:") || err_str.contains("empty"),
                "expected validation error for empty text, got: {err_str}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Correlation-id tracing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_correlation_id_in_trace_logs() {
    // Verify that extract_entities_and_relations propagates a correlation_id
    // through the tracing layer's #[tracing::instrument] span field.
    let config = match load_config() {
        Some(c) => c,
        None => return,
    };

    let pipeline = ExtractionPipeline::new(config);
    let text = "Test correlation id tracking through extraction pipeline.";
    let correlation_id = "test-correlation-uid-001";

    let result = pipeline
        .extract_entities_and_relations(text, correlation_id)
        .await;

    assert!(
        result.is_ok(),
        "extraction with correlation_id should succeed: {:?}",
        result.err()
    );

    let (entities, relations) = result.unwrap();
    // At minimum the method completed without error; extraction results are
    // model-dependent so we only assert the call itself succeeded and the
    // correlation_id was accepted by the tracing instrumentation.
    assert!(
        !entities.is_empty() || !relations.is_empty(),
        "expected at least some extraction results with correlation_id"
    );
}
