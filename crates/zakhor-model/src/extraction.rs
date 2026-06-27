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

use crate::ingestion::{EntityRef, Relation};

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
#[derive(Debug)]
pub enum ExtractionError {
    /// Failed to load the ONNX model or tokenizer.
    ModelLoad(String),
    /// ONNX inference or pipeline processing failed.
    Inference(String),
    /// Mapping extracted values back to Zakhor types failed.
    Mapping(String),
    /// The async blocking task itself panicked or was cancelled.
    TaskJoin(String),
}

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractionError::ModelLoad(msg) => write!(f, "model load: {}", msg),
            ExtractionError::Inference(msg) => write!(f, "inference: {}", msg),
            ExtractionError::Mapping(msg) => write!(f, "mapping: {}", msg),
            ExtractionError::TaskJoin(msg) => write!(f, "task join: {}", msg),
        }
    }
}

impl std::error::Error for ExtractionError {}

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

    /// Return a reference to the lazily initialised model state.
    fn get_or_init_model(&self) -> Result<Arc<Inner>, ExtractionError> {
        let mut guard = self.inner.lock().expect("extraction mutex poisoned");
        if let Some(ref inner) = *guard {
            return Ok(inner.clone());
        }

        let runtime_params = orp::params::RuntimeParameters::default();
        let model = orp::model::Model::new(&self.config.model_path, runtime_params)
            .map_err(|e| ExtractionError::ModelLoad(format!("ONNX model: {}", e)))?;

        let params = gliner::model::params::Parameters::default()
            .with_threshold(self.config.entity_threshold);

        let inner = Arc::new(Inner { model, params });
        *guard = Some(inner.clone());
        Ok(inner)
    }

    /// Extract named entities from `text`.
    ///
    /// Returns a list of [`EntityRef`] values with URIs formed as
    /// `http://zakhor/ns/entity/{class}` and labels set to the extracted span text.
    ///
    /// The ONNX model is loaded lazily on the first call and cached thereafter.
    pub async fn extract_entities(&self, text: &str) -> Result<Vec<EntityRef>, ExtractionError> {
        let inner = self.get_or_init_model()?;
        let config = self.config.clone();
        let text = text.to_string();

        spawn_blocking(move || {
            let entity_strs: Vec<&str> = config.entity_labels.iter().map(|s| s.as_str()).collect();

            let text_input =
                gliner::model::input::text::TextInput::from_str(&[&text], &entity_strs)
                    .map_err(|e| ExtractionError::Inference(format!("text input: {}", e)))?;

            let pipeline =
                gliner::model::pipeline::token::TokenPipeline::new(&config.tokenizer_path)
                    .map_err(|e| ExtractionError::ModelLoad(format!("tokenizer: {}", e)))?;

            let span_output: gliner::model::output::decoded::SpanOutput = inner
                .model
                .inference(text_input, &pipeline, &inner.params)
                .map_err(|e| ExtractionError::Inference(format!("NER inference: {}", e)))?;

            let entities: Vec<EntityRef> = span_output
                .spans
                .into_iter()
                .flat_map(|seq| {
                    seq.into_iter().map(|span| EntityRef {
                        uri: format!("http://zakhor/ns/entity/{}", span.class()),
                        label: span.text().to_string(),
                    })
                })
                .collect();

            Ok(entities)
        })
        .await
        .map_err(|e| ExtractionError::TaskJoin(format!("spawn_blocking: {}", e)))?
    }

    /// Extract relations between entities in `text`.
    ///
    /// Internally runs the GLiNER token pipeline (NER) first, then feeds the
    /// extracted spans into the relation extraction pipeline.
    ///
    /// `entities` is used to map relation subject/object texts to their Zakhor
    /// entity URIs. Any subject or object text that does not appear in the
    /// `entities` list falls back to
    /// `http://zakhor/ns/entity/{text}` as its URI.
    ///
    /// The ONNX model is loaded lazily on the first call and cached thereafter.
    pub async fn extract_relations(
        &self,
        text: &str,
        entities: &[EntityRef],
    ) -> Result<Vec<Relation>, ExtractionError> {
        let inner = self.get_or_init_model()?;
        let config = self.config.clone();
        let text = text.to_string();
        let entity_uris: Vec<(String, String)> = entities
            .iter()
            .map(|e| (e.label.clone(), e.uri.clone()))
            .collect();

        spawn_blocking(move || {
            // ---- Step 1: NER ----
            let entity_strs: Vec<&str> = config.entity_labels.iter().map(|s| s.as_str()).collect();

            let text_input =
                gliner::model::input::text::TextInput::from_str(&[&text], &entity_strs)
                    .map_err(|e| ExtractionError::Inference(format!("text input: {}", e)))?;

            let token_pipeline =
                gliner::model::pipeline::token::TokenPipeline::new(&config.tokenizer_path)
                    .map_err(|e| ExtractionError::ModelLoad(format!("tokenizer: {}", e)))?;

            let span_output: gliner::model::output::decoded::SpanOutput = inner
                .model
                .inference(text_input, &token_pipeline, &inner.params)
                .map_err(|e| ExtractionError::Inference(format!("NER inference: {}", e)))?;

            // ---- Step 2: Relation Extraction ----
            let mut relation_schema = gliner::model::input::relation::schema::RelationSchema::new();
            for label in &config.relation_labels {
                relation_schema.push(label);
            }

            let rel_pipeline = gliner::model::pipeline::relation::RelationPipeline::default(
                &config.tokenizer_path,
                &relation_schema,
            )
            .map_err(|e| ExtractionError::ModelLoad(format!("relation pipeline: {}", e)))?;

            let relation_output: gliner::model::output::relation::RelationOutput = inner
                .model
                .inference(span_output, &rel_pipeline, &inner.params)
                .map_err(|e| ExtractionError::Inference(format!("RE inference: {}", e)))?;

            // ---- Step 3: Map to Zakhor Relation types ----
            let lookup: std::collections::HashMap<&str, &str> = entity_uris
                .iter()
                .map(|(label, uri)| (label.as_str(), uri.as_str()))
                .collect();

            let relations: Vec<Relation> = relation_output
                .relations
                .into_iter()
                .flat_map(|seq| {
                    seq.into_iter().map(|rel| {
                        let subject_fallback = format!("http://zakhor/ns/entity/{}", rel.subject());
                        let object_fallback = format!("http://zakhor/ns/entity/{}", rel.object());

                        let subject_uri = lookup
                            .get(rel.subject())
                            .copied()
                            .unwrap_or(subject_fallback.as_str());
                        let object_uri = lookup
                            .get(rel.object())
                            .copied()
                            .unwrap_or(object_fallback.as_str());

                        Relation {
                            subject_uri: subject_uri.to_string(),
                            predicate_uri: format!("http://zakhor/ns/relation/{}", rel.class()),
                            object_uri: object_uri.to_string(),
                            label: rel.class().to_string(),
                        }
                    })
                })
                .collect();

            Ok(relations)
        })
        .await
        .map_err(|e| ExtractionError::TaskJoin(format!("spawn_blocking: {}", e)))?
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
        let err = ExtractionError::ModelLoad("file not found".into());
        let msg = format!("{}", err);
        assert!(msg.contains("model load: file not found"), "msg: {}", msg);

        let err = ExtractionError::Inference("timeout".into());
        assert!(format!("{}", err).contains("inference: timeout"));

        let err = ExtractionError::Mapping("bad label".into());
        assert!(format!("{}", err).contains("mapping: bad label"));

        let err = ExtractionError::TaskJoin("cancelled".into());
        assert!(format!("{}", err).contains("task join: cancelled"));
    }

    #[test]
    fn test_extraction_error_impl_error() {
        let err = ExtractionError::ModelLoad("fail".into());
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
}
