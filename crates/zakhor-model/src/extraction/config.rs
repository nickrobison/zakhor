use serde::{Deserialize, Serialize};
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

pub use zakhor_common::config::extraction_defaults::{
    DEFAULT_ENTITY_LABELS, DEFAULT_RELATION_LABELS,
};

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::new(),
            tokenizer_path: PathBuf::new(),
            entity_labels: DEFAULT_ENTITY_LABELS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            relation_labels: DEFAULT_RELATION_LABELS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            entity_threshold: 0.5,
            relation_threshold: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_config_default() {
        let config = ExtractionConfig::default();
        assert!(config.model_path.as_os_str().is_empty());
        assert!(config.tokenizer_path.as_os_str().is_empty());
        // GLiNER is zero-shot, so an empty label set makes extraction
        // impossible. A default install must still be able to run.
        assert!(
            !config.entity_labels.is_empty(),
            "a zero-config install must have labels to extract"
        );
        assert!(!config.relation_labels.is_empty());
        assert_eq!(config.entity_labels.len(), DEFAULT_ENTITY_LABELS.len());
        assert_eq!(config.relation_labels.len(), DEFAULT_RELATION_LABELS.len());
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
}
