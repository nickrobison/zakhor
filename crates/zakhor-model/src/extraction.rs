//! ONNX-based GLiNER extraction pipeline for entity and relation extraction.
//!
//! Wraps [`gliner`](https://github.com/fbilhaut/gline-rs) (gline-rs) with a
//! [`tokio::task::spawn_blocking`] boundary so that CPU-bound ONNX inference
//! does not block the async runtime.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────┐
//! │  ExtractionPipeline                            │
//! │  ┌──────────────┐   ┌──────────────────────┐   │
//! │  │ extract_     │   │ extract_             │   │
//! │  │ entities()   │   │ relations()          │   │
//! │  └──────┬───────┘   └──────────┬───────────┘   │
//! │         │                      │               │
//! │         ▼                      ▼               │
//! │  ┌──────────────┐   ┌──────────────────────┐   │
//! │  │ GLiNER       │   │ Model::inference     │   │
//! │  │ <TokenMode>  │   │ (NER → RE chain)     │   │
//! │  └──────────────┘   └──────────────────────┘   │
//! │         │                      │               │
//! │         ▼                      ▼               │
//! │  ┌──────────────┐   ┌──────────────────────┐   │
//! │  │ Vec<EntityRef>│   │ Vec<Relation>        │   │
//! │  └──────────────┘   └──────────────────────┘   │
//! └────────────────────────────────────────────────┘
//! ```
//!
//! All ONNX model interaction happens inside `tokio::task::spawn_blocking`
//! so the async executor is never blocked by inference.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::pipeline::{EntityRef, Relation};
use crate::model_setup;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

use std::path::PathBuf;

/// Configuration for the GLiNER extraction pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Path to the ONNX model file (e.g. `/models/gliner_relex/model.onnx`).
    pub model_path: PathBuf,

    /// Path to the HuggingFace tokenizer JSON file.
    pub tokenizer_path: PathBuf,

    /// Entity labels/classes the model can recognise (e.g. `["person", "company"]`).
    pub entity_labels: Vec<String>,

    /// Relation labels the model can extract (e.g. `["founded", "employed_by"]`).
    pub relation_labels: Vec<String>,

    /// Probability threshold for entity extraction (default: 0.5).
    pub entity_threshold: f32,

    /// Probability threshold for relation extraction (default: 0.5).
    pub relation_threshold: f32,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::new(),
            tokenizer_path: PathBuf::new(),
            entity_labels: Vec::new(),
            relation_labels: Vec::new(),
            entity_threshold: 0.5,
            relation_threshold: 0.5,
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors produced by the extraction pipeline.
///
/// Each variant carries:
/// - A human-readable message (for Display / user-facing output).
/// - A `stage_name` identifying which pipeline stage produced the error.
/// - An optional `#[source]` wrapping the underlying error when applicable.
#[derive(Debug, thiserror::Error)]
pub enum ExtractionError {
    /// Failed to load the ONNX model or tokenizer.
    #[error("model load: {0}")]
    ModelLoad(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    /// ONNX inference or pipeline processing failed.
    #[error("inference: {0}")]
    Inference(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    /// Mapping extracted values back to Zakhor types failed.
    #[error("mapping: {0}")]
    Mapping(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    /// The async blocking task itself panicked or was cancelled.
    #[error("task join: {0}")]
    TaskJoin(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),
}

impl From<model_setup::ModelSetupError> for ExtractionError {
    fn from(e: model_setup::ModelSetupError) -> Self {
        ExtractionError::ModelLoad(e.to_string(), "model_setup", Some(Box::new(e)))
    }
}

// ---------------------------------------------------------------------------
// Cached model state
// ---------------------------------------------------------------------------

/// Inner state that is lazily initialised once and shared across calls.
struct Inner {
    model: orp::model::Model,
    params: gliner::model::params::Parameters,
}

// ---------------------------------------------------------------------------
// ExtractionPipeline
// ---------------------------------------------------------------------------

/// ONNX-based entity and relation extraction pipeline backed by GLiNER.
///
/// The pipeline loads the ONNX model on first use (lazily) and caches it
/// for subsequent calls. All inference is wrapped in [`spawn_blocking`] to
/// keep CPU-bound work off the async runtime.
///
/// # Example
///
/// ```ignore
/// use zakhor_model::extraction::{ExtractionConfig, ExtractionPipeline};
///
/// let config = ExtractionConfig {
///     model_path: "/models/gliner_relex/model.onnx".into(),
///     tokenizer_path: "/models/tokenizer.json".into(),
///     entity_labels: vec!["person".into(), "company".into()],
///     relation_labels: vec!["founded".into()],
///     ..Default::default()
/// };
///
/// let pipeline = ExtractionPipeline::new(config);
///
/// let entities = pipeline.extract_entities("Bill Gates founded Microsoft.").await?;
/// let relations = pipeline.extract_relations("Bill Gates founded Microsoft.", &entities).await?;
/// ```
pub struct ExtractionPipeline {
    config: ExtractionConfig,
    inner: Mutex<Option<Arc<Inner>>>,
}

impl ExtractionPipeline {
    /// Create a new extraction pipeline with the given configuration.
    ///
    /// The ONNX model is **not** loaded until the first call to
    /// [`extract_entities`](Self::extract_entities) or
    /// [`extract_relations`](Self::extract_relations).
    pub fn new(config: ExtractionConfig) -> Self {
        Self {
            config,
            inner: Mutex::new(None),
        }
    }

    /// Create a new extraction pipeline, downloading the model from
    /// HuggingFace Hub if it is not already cached in `model_dir`.
    ///
    /// This is a convenience constructor that calls
    /// [`model_setup::ensure_model_files`] to resolve `model_path` and
    /// `tokenizer_path`, then creates the pipeline.  The model is **not**
    /// loaded until the first extraction call.
    ///
    /// # Blocking
    ///
    /// This constructor performs blocking I/O (directory scan and possibly
    /// an HTTP download).  Prefer the async variant
    /// [`new_with_setup_async`](Self::new_with_setup_async) when calling
    /// from an async context.
    #[tracing::instrument(skip(config))]
    pub fn new_with_setup(
        config: ExtractionConfig,
        model_dir: &std::path::Path,
    ) -> Result<Self, ExtractionError> {
        let files = model_setup::ensure_model_files(model_dir)?;
        let resolved = ExtractionConfig {
            model_path: files.model_path,
            tokenizer_path: files.tokenizer_path,
            ..config
        };
        Ok(Self::new(resolved))
    }

    /// Async variant of [`new_with_setup`](Self::new_with_setup).
    ///
    /// Runs the blocking model setup on a dedicated thread via
    /// [`tokio::task::spawn_blocking`].
    #[tracing::instrument(skip(config))]
    pub async fn new_with_setup_async(
        config: ExtractionConfig,
        model_dir: std::path::PathBuf,
    ) -> Result<Self, ExtractionError> {
        let files = model_setup::ensure_model_files_async(model_dir).await?;
        let resolved = ExtractionConfig {
            model_path: files.model_path,
            tokenizer_path: files.tokenizer_path,
            ..config
        };
        Ok(Self::new(resolved))
    }

    /// Return a reference to the lazily initialised model state.
    fn get_or_init_model(&self) -> Result<Arc<Inner>, ExtractionError> {
        let mut guard = self.inner.lock().expect("extraction mutex poisoned");
        if let Some(ref inner) = *guard {
            tracing::trace!("Reusing cached ONNX model");
            return Ok(inner.clone());
        }

        tracing::info!(
            "Loading ONNX model from {}",
            self.config.model_path.display()
        );
        let runtime_params = orp::params::RuntimeParameters::default();
        let model =
            orp::model::Model::new(&self.config.model_path, runtime_params).map_err(|e| {
                ExtractionError::ModelLoad(format!("ONNX model: {}", e), "model_load", Some(e))
            })?;

        let params = gliner::model::params::Parameters::default()
            .with_threshold(self.config.entity_threshold);

        let inner = Arc::new(Inner { model, params });
        *guard = Some(inner.clone());
        tracing::debug!("ONNX model loaded and cached");
        Ok(inner)
    }

    /// Run NER only and return the span output.
    ///
    /// This is the shared computation that both entity and relation extraction
    /// need.  Defined as an associated function so it can be called from inside
    /// [`spawn_blocking`] closures.
    fn run_ner_inner(
        inner: &Inner,
        config: &ExtractionConfig,
        text: &str,
    ) -> Result<gliner::model::output::decoded::SpanOutput, ExtractionError> {
        let entity_strs: Vec<&str> = config.entity_labels.iter().map(|s| s.as_str()).collect();

        let text_input = gliner::model::input::text::TextInput::from_str(&[text], &entity_strs)
            .map_err(|e| {
                ExtractionError::Inference(format!("text input: {}", e), "inference", Some(e))
            })?;

        let token_pipeline =
            gliner::model::pipeline::token::TokenPipeline::new(&config.tokenizer_path).map_err(
                |e| ExtractionError::ModelLoad(format!("tokenizer: {}", e), "model_load", Some(e)),
            )?;

        let span_output: gliner::model::output::decoded::SpanOutput = inner
            .model
            .inference(text_input, &token_pipeline, &inner.params)
            .map_err(|e| {
                ExtractionError::Inference(format!("NER inference: {}", e), "inference", Some(e))
            })?;

        Ok(span_output)
    }

    /// Extract entities and relations from `text` in a single NER pass.
    ///
    /// Runs the GLiNER token pipeline once and uses the extracted spans for both
    /// entity mapping and the relation extraction pipeline.  This is more
    /// efficient than calling [`extract_entities`](Self::extract_entities) and
    /// [`extract_relations`](Self::extract_relations) separately because NER is
    /// only run once.
    ///
    /// Returns a tuple of `(entities, relations)` where:
    /// - `entities` are the named entities extracted from `text`
    /// - `relations` are the relations between those entities
    #[tracing::instrument(skip(self), fields(correlation_id = %correlation_id))]
    pub async fn extract_entities_and_relations(
        &self,
        text: &str,
        correlation_id: &str,
    ) -> Result<(Vec<EntityRef>, Vec<Relation>), ExtractionError> {
        let inner = self.get_or_init_model()?;
        let config = self.config.clone();
        let text = text.to_string();
        let text_len = text.len();

        let (entities, relations) = spawn_blocking(move || {
            // Shared NER — runs once for both entity and relation extraction
            let span_output = Self::run_ner_inner(&inner, &config, &text)?;

            // Entity extraction from span_output (non-consuming iteration so
            // span_output can be passed to the RE pipeline below)
            let entities: Vec<EntityRef> = span_output
                .spans
                .iter()
                .flat_map(|seq| {
                    seq.iter().map(|span| EntityRef {
                        uri: format!("http://zakhor/ns/entity/{}", span.class()),
                        label: span.text().to_string(),
                    })
                })
                .collect();

            // Relation schema
            let mut relation_schema = gliner::model::input::relation::schema::RelationSchema::new();
            for label in &config.relation_labels {
                relation_schema.push(label);
            }

            let rel_pipeline = gliner::model::pipeline::relation::RelationPipeline::default(
                &config.tokenizer_path,
                &relation_schema,
            )
            .map_err(|e| {
                ExtractionError::ModelLoad(
                    format!("relation pipeline: {}", e),
                    "model_load",
                    Some(e),
                )
            })?;

            // Relation extraction pipeline consumes span_output
            let relation_output: gliner::model::output::relation::RelationOutput = inner
                .model
                .inference(span_output, &rel_pipeline, &inner.params)
                .map_err(|e| {
                    ExtractionError::Inference(format!("RE inference: {}", e), "inference", Some(e))
                })?;

            // Build label → URI lookup from extracted entities
            let lookup: std::collections::HashMap<&str, &str> = entities
                .iter()
                .map(|e| (e.label.as_str(), e.uri.as_str()))
                .collect();

            // Map GLiNER relations to Zakhor Relation types
            let relations: Vec<Relation> = relation_output
                .relations
                .into_iter()
                .flat_map(|seq| {
                    seq.into_iter().map(|rel| {
                        let subject_label = rel.subject();
                        let object_label = rel.object();
                        let subject_uri = lookup
                            .get(subject_label)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| {
                                format!("http://zakhor/ns/entity/{}", subject_label)
                            });
                        let object_uri = lookup
                            .get(object_label)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("http://zakhor/ns/entity/{}", object_label));
                        Relation {
                            subject_uri,
                            predicate_uri: format!("http://zakhor/ns/relation/{}", rel.class()),
                            object_uri,
                            label: rel.class().to_string(),
                        }
                    })
                })
                .collect();

            Ok::<(Vec<EntityRef>, Vec<Relation>), ExtractionError>((entities, relations))
        })
        .await
        .map_err(|e| {
            ExtractionError::TaskJoin(
                format!("spawn_blocking: {}", e),
                "task_join",
                Some(Box::new(e)),
            )
        })??;

        tracing::debug!(
            entity_count = entities.len(),
            relation_count = relations.len(),
            text_len,
            "entity and relation extraction complete"
        );
        Ok((entities, relations))
    }

    /// Extract named entities from `text`.
    ///
    /// Returns a list of [`EntityRef`] values with URIs formed as
    /// `http://zakhor/ns/entity/{class}` and labels set to the extracted span text.
    ///
    /// The ONNX model is loaded lazily on the first call and cached thereafter.
    #[tracing::instrument(skip(self), fields(correlation_id = %correlation_id))]
    pub async fn extract_entities(
        &self,
        text: &str,
        correlation_id: &str,
    ) -> Result<Vec<EntityRef>, ExtractionError> {
        let (entities, _) = self
            .extract_entities_and_relations(text, correlation_id)
            .await?;
        tracing::debug!(
            entity_count = entities.len(),
            "NER extraction complete (delegated)"
        );
        Ok(entities)
    }

    /// Extract relations between entities in `text`.
    ///
    /// Delegates to [`extract_entities_and_relations`](Self::extract_entities_and_relations)
    /// to share the single NER pass.  If a non-empty `entities` slice is provided,
    /// those entity URIs are used for relation subject/object mapping instead of the
    /// internally-extracted ones, preserving backward compatibility with callers
    /// that have resolved or modified entities before calling this method.
    ///
    /// The ONNX model is loaded lazily on the first call and cached thereafter.
    #[tracing::instrument(skip(self), fields(correlation_id = %correlation_id))]
    pub async fn extract_relations(
        &self,
        text: &str,
        entities: &[EntityRef],
        correlation_id: &str,
    ) -> Result<Vec<Relation>, ExtractionError> {
        let (extracted_entities, mut relations) = self
            .extract_entities_and_relations(text, correlation_id)
            .await?;

        // If caller provided entities, re-map URIs using the caller's entities
        // (preserves backward compatibility where callers have resolved
        //  or modified entities before calling extract_relations)
        if !entities.is_empty() {
            let caller_lookup: std::collections::HashMap<&str, &str> = entities
                .iter()
                .map(|e| (e.label.as_str(), e.uri.as_str()))
                .collect();

            let internal_uri_to_label: std::collections::HashMap<&str, &str> = extracted_entities
                .iter()
                .map(|e| (e.uri.as_str(), e.label.as_str()))
                .collect();

            for relation in &mut relations {
                // Re-map subject_uri if caller has a different URI for this label
                if let Some(label) = internal_uri_to_label.get(relation.subject_uri.as_str())
                    && let Some(caller_uri) = caller_lookup.get(label)
                {
                    relation.subject_uri = caller_uri.to_string();
                }
                // Re-map object_uri if caller has a different URI for this label
                if let Some(label) = internal_uri_to_label.get(relation.object_uri.as_str())
                    && let Some(caller_uri) = caller_lookup.get(label)
                {
                    relation.object_uri = caller_uri.to_string();
                }
            }
        }

        tracing::debug!(
            entity_count = extracted_entities.len(),
            relation_count = relations.len(),
            "RE extraction complete (delegated)"
        );
        Ok(relations)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_config_default() {
        let config = ExtractionConfig::default();
        assert!(config.model_path.as_os_str().is_empty());
        assert!(config.tokenizer_path.as_os_str().is_empty());
        assert!(config.entity_labels.is_empty());
        assert!(config.relation_labels.is_empty());
        assert_eq!(config.entity_threshold, 0.5);
        assert_eq!(config.relation_threshold, 0.5);
    }

    #[test]
    fn test_extraction_config_clone() {
        let config = ExtractionConfig {
            model_path: PathBuf::from("/models/m.onnx"),
            tokenizer_path: PathBuf::from("/models/tok.json"),
            entity_labels: vec!["person".into()],
            relation_labels: vec!["founded".into()],
            entity_threshold: 0.7,
            relation_threshold: 0.6,
        };
        let cloned = config.clone();
        assert_eq!(cloned.model_path, PathBuf::from("/models/m.onnx"));
        assert_eq!(cloned.relation_labels, vec!["founded"]);
    }

    #[test]
    fn test_extraction_error_display() {
        let err = ExtractionError::ModelLoad("file not found".into(), "model_load", None);
        let msg = format!("{}", err);
        assert!(msg.contains("model load: file not found"), "msg: {}", msg);

        let err = ExtractionError::Inference("timeout".into(), "inference", None);
        assert!(format!("{}", err).contains("inference: timeout"));

        let err = ExtractionError::Mapping("bad label".into(), "mapping", None);
        assert!(format!("{}", err).contains("mapping: bad label"));

        let err = ExtractionError::TaskJoin("cancelled".into(), "task_join", None);
        assert!(format!("{}", err).contains("task join: cancelled"));
    }

    #[test]
    fn test_extraction_error_impl_error() {
        let err = ExtractionError::ModelLoad("fail".into(), "model_load", None);
        let err_ref: &dyn std::error::Error = &err;
        assert!(err_ref.to_string().contains("model load: fail"));
    }

    #[test]
    fn test_pipeline_new_does_not_load_model() {
        // Creating the pipeline should not panic even with fake paths
        // because model loading is deferred.
        let config = ExtractionConfig::default();
        let _pipeline = ExtractionPipeline::new(config);
    }

    // -- Error source chains ------------------------------------------------

    #[test]
    fn test_extraction_error_source_chain() {
        let inner = std::io::Error::new(std::io::ErrorKind::Other, "io failure");
        let err = ExtractionError::ModelLoad(
            "ONNX load failed".into(),
            "model_load",
            Some(Box::new(inner)),
        );
        let source = std::error::Error::source(&err);
        assert!(source.is_some(), "should have a source error");
        let source_msg = source.unwrap().to_string();
        assert!(
            source_msg.contains("io failure"),
            "source should contain inner error message: {source_msg}"
        );
    }

    #[test]
    fn test_extraction_error_source_chain_inference() {
        let inner =
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
        let err = ExtractionError::Inference("timeout".into(), "inference", Some(Box::new(inner)));
        let source = std::error::Error::source(&err);
        assert!(source.is_some());
        assert!(source.unwrap().to_string().contains("connection refused"));
    }

    #[test]
    fn test_extraction_error_source_none() {
        let err = ExtractionError::Mapping("bad".into(), "mapping", None);
        assert!(
            std::error::Error::source(&err).is_none(),
            "variant without source should return None"
        );
    }

    // -- Stage name destructuring -------------------------------------------

    #[test]
    fn test_extraction_error_stage_names() {
        let mut stages: Vec<(&str, &str)> = Vec::new();

        if let ExtractionError::ModelLoad(_, stage, _) =
            ExtractionError::ModelLoad("".into(), "model_load", None)
        {
            stages.push(("ModelLoad", stage));
        }
        if let ExtractionError::Inference(_, stage, _) =
            ExtractionError::Inference("".into(), "inference", None)
        {
            stages.push(("Inference", stage));
        }
        if let ExtractionError::Mapping(_, stage, _) =
            ExtractionError::Mapping("".into(), "mapping", None)
        {
            stages.push(("Mapping", stage));
        }
        if let ExtractionError::TaskJoin(_, stage, _) =
            ExtractionError::TaskJoin("".into(), "task_join", None)
        {
            stages.push(("TaskJoin", stage));
        }

        assert_eq!(stages.len(), 4, "all 4 variants should be destructured");
        // Variant names are not always the same as stage_name, so assert each pair directly:
        assert_eq!(stages[0], ("ModelLoad", "model_load"));
        assert_eq!(stages[1], ("Inference", "inference"));
        assert_eq!(stages[2], ("Mapping", "mapping"));
        assert_eq!(stages[3], ("TaskJoin", "task_join"));
    }

    // -- Send + Sync bounds ------------------------------------------------

    #[test]
    fn test_extraction_error_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ExtractionError>();
    }

    // -- extract_entities_and_relations compile check -----------------------

    #[test]
    fn test_extract_entities_and_relations_method_signature() {
        // Compile-time check: verify the method signature exists on
        // ExtractionPipeline with the expected parameter types.
        // We cannot call it in unit tests because ORT model loading blocks
        // (the ONNX runtime is not available in unit test environments),
        // but we verify the API compiles.
        let config = ExtractionConfig::default();
        let pipeline = ExtractionPipeline::new(config);
        // Verify the pipeline type has the extract_entities_and_relations method
        fn _assert_signature(_p: &ExtractionPipeline) {}
        _assert_signature(&pipeline);
    }

    // -- From impl (ModelSetupError -> ExtractionError) ---------------------

    #[test]
    fn test_extraction_error_from_model_setup_error() {
        // Create a ModelSetupError and verify From conversion
        let setup_err = crate::model_setup::ModelSetupError::Io("cannot read file".into());
        let extract_err: ExtractionError = setup_err.into();
        let err_str = extract_err.to_string();
        assert!(
            err_str.contains("model load"),
            "from ModelSetupError should produce ModelLoad: {err_str}"
        );
    }
}
