use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub endpoint: String,
    pub model: String,
    pub extraction_timeout_secs: u64,
    pub confidence_threshold: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntityResolutionConfig {
    pub alias_threshold: f32,
    pub tantivy_threshold: f32,
    pub fastembed_threshold: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RankingConfig {
    pub graph_importance_weight: f32,
    pub provenance_quality_weight: f32,
    pub lexical_weight: f32,
    pub semantic_weight: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodeIndexingConfig {
    pub max_parallel_parsers: usize,
    pub repo_poll_interval_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolCaptureConfig {
    pub max_evidence_per_decision: usize,
    pub session_timeout_minutes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackgroundConfig {
    pub worker_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Default, Deserialize)]
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
