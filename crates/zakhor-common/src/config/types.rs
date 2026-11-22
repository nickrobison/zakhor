use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("malformed TOML in config file {path}: {source}")]
    MalformedToml {
        path: std::path::PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("invalid configuration value {key} = {value}: {message}")]
    Invalid {
        key: String,
        value: String,
        message: String,
    },

    #[error("failed to load config from {path}: {source}")]
    Load {
        path: std::path::PathBuf,
        #[source]
        source: Box<figment::Error>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub database: DatabaseConfig,
    #[serde(default)]
    pub http: HttpConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub entity_resolution: EntityResolutionConfig,
    #[serde(default)]
    pub ranking: RankingConfig,
    #[serde(default)]
    pub code_indexing: CodeIndexingConfig,
    #[serde(default)]
    pub tool_capture: ToolCaptureConfig,
    #[serde(default)]
    pub background: BackgroundConfig,
    #[serde(default)]
    pub extraction: ExtractionConfig,
    #[serde(default)]
    pub embedding: EmbeddingConfig,
    #[serde(default)]
    pub models: ModelsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    pub endpoint: String,
    pub model: String,
    pub extraction_timeout_secs: u64,
    pub confidence_threshold: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntityResolutionConfig {
    pub alias_threshold: f32,
    pub tantivy_threshold: f32,
    pub fastembed_threshold: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RankingConfig {
    pub graph_importance_weight: f32,
    pub provenance_quality_weight: f32,
    pub lexical_weight: f32,
    pub semantic_weight: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeIndexingConfig {
    pub max_parallel_parsers: usize,
    pub repo_poll_interval_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolCaptureConfig {
    pub max_evidence_per_decision: usize,
    pub session_timeout_minutes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackgroundConfig {
    pub worker_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractionConfig {
    pub model_path: PathBuf,
    pub tokenizer_path: PathBuf,
    /// Directory for auto-downloading the model from HuggingFace Hub.
    ///
    /// When non-empty and `model_path` is empty, the model is downloaded
    /// automatically on startup via `hf-hub`.
    #[serde(default)]
    pub model_dir: PathBuf,
    pub entity_labels: Vec<String>,
    pub relation_labels: Vec<String>,
    pub entity_threshold: f32,
    pub relation_threshold: f32,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EmbeddingConfig {
    /// Enable Fastembed semantic search index.
    ///
    /// Disabled by default because Fastembed model initialisation (model
    /// download + ONNX runtime warm-up) can hang or crash in some
    /// environments. Set to `true` in `zakhor.toml` when semantic search
    /// is desired.
    #[serde(default)]
    pub enabled: bool,
}

impl Config {
    /// Validate the configuration, failing with an error that names the
    /// offending key and value when any setting falls outside its allowed range.
    ///
    /// Runs on every configuration load path (built-in defaults, TOML file, or
    /// `ZAKHOR_*` environment overrides) so misconfiguration surfaces loudly
    /// instead of silently degrading to defaults or producing broken behavior.
    pub fn validate(&self) -> Result<(), ConfigError> {
        for (key, value) in [
            ("llm.confidence_threshold", self.llm.confidence_threshold),
            (
                "entity_resolution.alias_threshold",
                self.entity_resolution.alias_threshold,
            ),
            (
                "entity_resolution.tantivy_threshold",
                self.entity_resolution.tantivy_threshold,
            ),
            (
                "entity_resolution.fastembed_threshold",
                self.entity_resolution.fastembed_threshold,
            ),
            (
                "extraction.entity_threshold",
                self.extraction.entity_threshold,
            ),
            (
                "extraction.relation_threshold",
                self.extraction.relation_threshold,
            ),
            (
                "ranking.graph_importance_weight",
                self.ranking.graph_importance_weight,
            ),
            (
                "ranking.provenance_quality_weight",
                self.ranking.provenance_quality_weight,
            ),
            ("ranking.lexical_weight", self.ranking.lexical_weight),
            ("ranking.semantic_weight", self.ranking.semantic_weight),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(ConfigError::Invalid {
                    key: key.to_string(),
                    value: value.to_string(),
                    message: format!("{key} must be a probability between 0 and 1, got {value}"),
                });
            }
        }

        if !(1..=u16::MAX).contains(&self.http.port) {
            return Err(ConfigError::Invalid {
                key: "http.port".to_string(),
                value: self.http.port.to_string(),
                message: format!(
                    "http.port must be between 1 and 65535, got {}",
                    self.http.port
                ),
            });
        }

        if self.background.worker_count == 0 {
            return Err(ConfigError::Invalid {
                key: "background.worker_count".to_string(),
                value: self.background.worker_count.to_string(),
                message: "background.worker_count must be greater than 0".to_string(),
            });
        }

        Ok(())
    }
}

/// Configuration for downloaded ML model caches (FastEmbed + GLiNER share one directory).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ModelsConfig {
    /// Shared on-disk cache directory for downloaded ML models.
    /// When unset/empty: `$XDG_CACHE_HOME/zakhor/models` (platform cache fallback via `dirs`).
    #[serde(default)]
    pub cache_dir: Option<PathBuf>,
}
