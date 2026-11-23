//! Integration tests for the parallel extraction + ingestion pipeline.
//!
//! These tests load a real ONNX model from disk and optionally connect to a
//! Tracker SPARQL database. A test that needs the model returns early when no
//! model can be found, so the suite stays green on a machine without it while
//! still running everywhere the model is present.
//!
//! The extraction assertions require the success path: the pinned `gline-rs`
//! fork now decodes the shipped `nickrobison/gliner-relex-onnx` export end to
//! end (entity spans plus native relation decode, see issue #94), so a failing
//! extraction is a real regression and must fail the test rather than be
//! papered over. Failures that do surface are still required to be actionable —
//! the binding's bare `unexpected logits shape` must never reach a caller, and
//! the shipped model's declared interface must be one the binding accepts.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use zakhor_model::extraction::{ExtractionConfig, ExtractionPipeline};
use zakhor_model::pipeline::IngestionPipeline;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Locate a usable GLiNER model without requiring configuration or a download.
///
/// `GLINER_MODEL_PATH` wins when set. Otherwise the cache directory is resolved
/// with the same helper the application uses, then scanned with the library's
/// own resolver, so these tests cannot disagree with the app about where the
/// model lives. The old `/models/gliner-relex/...` fallback existed on no
/// machine, so every test here returned early and none had ever run.
fn discover_model() -> Option<(PathBuf, PathBuf)> {
    if let Ok(p) = std::env::var("GLINER_MODEL_PATH") {
        let model = PathBuf::from(p);
        let tokenizer = PathBuf::from(std::env::var("GLINER_TOKENIZER_PATH").ok()?);
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

/// Returns `Err` when the Tracker runtime library is not available (e.g. not
/// Open the integration-test store, converting any open failure into a
/// caller-visible `Err` (tests skip rather than fail when the Tracker C
/// library or GLib resources are missing).
fn try_sparql_connection() -> Result<tracker::SparqlConnection, String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_dir = std::env::temp_dir().join(format!("zakhor-integration-{timestamp}"));

    zakhor_storage::tracker_db::init_db(
        tmp_dir
            .to_str()
            .ok_or_else(|| "temp path is not valid UTF-8".to_string())?,
    )
    .map_err(|e| format!("Tracker runtime not available: {e}"))
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
        .expect("entity + relation extraction should succeed end to end (#94)");

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
        // A typed URI is non-empty by construction; assert it round-trips through
        // the IRI parser, which is the invariant that actually matters.
        let reparsed = oxiri::Iri::parse(entity.uri.as_str()).expect("entity URI must reparse");
        assert_eq!(reparsed.as_str(), entity.uri.as_str());
        assert!(!entity.label.is_empty(), "entity label must not be empty");
    }

    // Verify relation structure
    for relation in &relations {
        assert!(
            oxiri::Iri::parse(relation.subject_uri.as_str()).is_ok(),
            "relation subject must not be empty"
        );
        assert!(
            oxiri::Iri::parse(relation.predicate_uri.as_str()).is_ok(),
            "relation predicate must not be empty"
        );
        assert!(
            oxiri::Iri::parse(relation.object_uri.as_str()).is_ok(),
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

    let (entities, relations) = pipeline
        .extract_entities_and_relations(text, "test-smoke-001")
        .await
        .expect("extraction should succeed end to end (#94)");

    assert!(
        !entities.is_empty() || !relations.is_empty(),
        "a successful extraction must yield something from: {text}"
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
    // 4. Call extract_and_ingest
    // 5. Verify it returns IngestResult
    let config = match load_config() {
        Some(c) => c,
        None => return,
    };

    let conn = match try_sparql_connection() {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("skipping SPARQL-dependent test: {e}");
            return;
        }
    };

    let extraction = ExtractionPipeline::new(config);
    let mut pipeline = IngestionPipeline::with_sync_manager(None);

    let text = "John works at Google in Mountain View.";
    let correlation_id = "test-functional-001";

    let (entities, relations) = extraction
        .extract_entities_and_relations(text, correlation_id)
        .await
        .expect("extraction should succeed end to end (#94)");
    assert!(
        !entities.is_empty(),
        "expected at least one entity in: {text}"
    );
    assert!(
        !relations.is_empty(),
        "expected at least one relation in: {text}"
    );

    let result = pipeline
        .extract_and_ingest(conn, text, &extraction, correlation_id)
        .await;

    match result {
        Ok(ingest_result) => {
            assert!(
                ingest_result
                    .observation_uri
                    .as_str()
                    .starts_with("urn:uuid:"),
                "observation_uri should be a URN UUID: {}",
                ingest_result.observation_uri.as_str()
            );
            assert!(
                ingest_result.triple_count > 0,
                "expected at least one triple: {}",
                ingest_result.triple_count
            );
        }
        Err(e) => {
            // If the pipeline fails at persist (e.g. ontology mismatch in CI),
            // verify it's not a validation error — extraction and validation
            // have already succeeded by this point.
            let err_str = e.to_string();
            assert!(
                !err_str.contains("validation:"),
                "validation should not fail with non-empty text: {err_str}"
            );
            // What must never happen is a failure the caller cannot act on, so
            // the opaque binding error is rejected explicitly.
            assert!(
                !err_str.contains("unexpected logits shape"),
                "the binding's opaque shape error must not reach the caller: {err_str}"
            );
            // Allow persist / sync errors when running in environments
            // without a fully-working Tracker in-process store.
            let is_persist_or_sync = err_str.contains("persist:")
                || err_str.contains("sync:")
                || err_str.contains("SPARQL");
            assert!(
                is_persist_or_sync,
                "unexpected error (expected persist/sync only): {err_str}"
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
    // Verify that extract_and_ingest returns a validation error when the
    // text is empty — validation is Stage 1, before any SPARQL operation.
    let config = match load_config() {
        Some(c) => c,
        None => return,
    };

    let conn = match try_sparql_connection() {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("skipping SPARQL-dependent test: {e}");
            return;
        }
    };

    let extraction = ExtractionPipeline::new(config);
    let mut pipeline = IngestionPipeline::with_sync_manager(None);

    let result = pipeline
        .extract_and_ingest(conn, "", &extraction, "test-empty-001")
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

    let (entities, relations) = pipeline
        .extract_entities_and_relations(text, correlation_id)
        .await
        .expect("extraction should succeed end to end (#94)");

    // At minimum the method completed without error; extraction results are
    // model-dependent so we only assert the call itself succeeded and the
    // correlation_id was accepted by the tracing instrumentation.
    assert!(
        !entities.is_empty() || !relations.is_empty(),
        "expected at least some extraction results with correlation_id"
    );
}

/// Pins the shipped model's declared interface.
///
/// The extraction bug used to be invisible because nothing recorded what the
/// model actually declares. This test reads the real ONNX file when present and
/// asserts the properties that decide whether the binding can decode it: a
/// `logits` output of rank 4, native relation outputs, and — for the token
/// RELEX export — the token-style inputs the fork's RELEX pipeline feeds. It
/// documents reality rather than asserting a wish, so it keeps passing after a
/// model swap and will show exactly what changed.
#[test]
fn test_shipped_model_declares_a_decodable_interface() {
    let Some((model_path, tokenizer_path)) = discover_model() else {
        eprintln!("skipping: no GLiNER model found");
        return;
    };
    assert!(
        tokenizer_path.exists(),
        "tokenizer must sit beside the model: {}",
        tokenizer_path.display()
    );

    let contract = zakhor_model::extraction::compat::read_contract(&model_path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", model_path.display()));

    eprintln!(
        "model {} declares inputs {:?} outputs {:?} logits_rank {:?} has_relations {}",
        model_path.display(),
        contract.inputs,
        contract.outputs,
        contract.logits_rank,
        contract.has_relations
    );

    assert!(
        contract.outputs.iter().any(|o| o == "logits"),
        "a GLiNER model must expose `logits`, got {:?}",
        contract.outputs
    );
    assert_eq!(
        contract.logits_rank,
        Some(4),
        "both gline-rs decoders require a rank-4 `logits` tensor"
    );
    assert!(
        contract.has_relations,
        "the shipped relex model must emit native relation outputs, got {:?}",
        contract.outputs
    );

    // The shipped model must be the static-capacity export: token-style
    // inputs plus the `span_idx`/`span_mask` slot inputs the relation head
    // scores pairs over. The legacy export (token inputs only) baked its
    // relation outputs into constants, which is exactly what #94 is about.
    let takes_span_inputs = ["span_idx", "span_mask"]
        .iter()
        .all(|n| contract.inputs.iter().any(|i| i == n));
    assert!(
        takes_span_inputs,
        "the shipped relex model must expose the static-capacity slot inputs \
         (span_idx/span_mask), got {:?}; the legacy export's relation outputs \
         are structurally constant and cannot produce relations",
        contract.inputs
    );
    assert!(
        ["input_ids", "attention_mask"]
            .iter()
            .all(|n| contract.inputs.iter().any(|i| i == n)),
        "the shipped relex model must feed the token-mode decoder, got {:?}",
        contract.inputs
    );

    // The whole point of the fix (#94): the shipped model is now a supported
    // configuration whose relations actually work — not merely a signature the
    // decoder accepts.
    assert_eq!(
        zakhor_model::extraction::compat::explain_incompatibility(&model_path),
        None,
        "the shipped relex model must be accepted by the binding's decoder contract"
    );
    assert_eq!(
        zakhor_model::extraction::compat::relation_support(&model_path),
        Some(zakhor_model::extraction::compat::RelationSupport::Native),
        "the shipped relex model must expose data-driven (native) relations"
    );
}
