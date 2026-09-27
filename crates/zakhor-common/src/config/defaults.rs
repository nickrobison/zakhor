use super::types::*;
use figment::Figment;
use figment::value::{Uncased, UncasedStr};
use std::path::PathBuf;

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434/api/generate".to_string(),
            model: "llama3".to_string(),
            extraction_timeout_secs: 30,
            confidence_threshold: 0.7,
        }
    }
}

impl Default for EntityResolutionConfig {
    fn default() -> Self {
        Self {
            alias_threshold: 1.0,
            tantivy_threshold: 0.85,
            fastembed_threshold: 0.78,
        }
    }
}

impl Default for RankingConfig {
    fn default() -> Self {
        Self {
            graph_importance_weight: 0.3,
            provenance_quality_weight: 0.2,
            lexical_weight: 0.3,
            semantic_weight: 0.2,
        }
    }
}

impl Default for CodeIndexingConfig {
    fn default() -> Self {
        Self {
            max_parallel_parsers: 4,
            repo_poll_interval_secs: 300,
        }
    }
}

impl Default for ToolCaptureConfig {
    fn default() -> Self {
        Self {
            max_evidence_per_decision: 50,
            session_timeout_minutes: 60,
        }
    }
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self { worker_count: 2 }
    }
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::default(),
            tokenizer_path: PathBuf::default(),
            model_dir: PathBuf::default(),
            entity_labels: Vec::new(),
            relation_labels: Vec::new(),
            entity_threshold: 0.5,
            relation_threshold: 0.5,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                path: PathBuf::from("./zakhor-db"),
            },
            http: HttpConfig::default(),
            llm: LlmConfig::default(),
            entity_resolution: EntityResolutionConfig::default(),
            ranking: RankingConfig::default(),
            code_indexing: CodeIndexingConfig::default(),
            tool_capture: ToolCaptureConfig::default(),
            background: BackgroundConfig::default(),
            extraction: ExtractionConfig::default(),
            embedding: EmbeddingConfig::default(),
            models: ModelsConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration layered from: built-in defaults < TOML file at `path` < `ZAKHOR_*` env vars.
    ///
    /// If the TOML file is missing or malformed, defaults plus env vars are used
    /// (a warning is logged). Existing callers that don't care about the path can
    /// continue to use [`Config::load`].
    pub fn load_from(path: &std::path::Path) -> Self {
        use figment::providers::{Env, Format, Serialized, Toml};

        let mut figment = Figment::new().merge(Serialized::defaults(Config::default()));

        // Only merge TOML layer if path is not empty
        if !path.as_os_str().is_empty() {
            figment = figment.merge(Toml::file(path));
        }

        let result: Result<Self, _> = figment
            .merge(Env::prefixed("ZAKHOR_").map(map_env_key))
            .extract();

        match result {
            Ok(config) => config,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Failed to load config file; using defaults + env vars"
                );
                Self::default().apply_env()
            }
        }
    }

    /// Load configuration from `./zakhor.toml` (preserved for backward compatibility).
    pub fn load() -> Self {
        Self::load_from(std::path::Path::new("./zakhor.toml"))
    }

    fn apply_env(mut self) -> Self {
        if let Ok(path) = std::env::var("ZAKHOR_DB_PATH") {
            self.database.path = PathBuf::from(path);
        }
        if let Ok(host) = std::env::var("ZAKHOR_HTTP_HOST") {
            self.http.host = host;
        }
        if let Ok(port) = std::env::var("ZAKHOR_HTTP_PORT")
            && let Ok(p) = port.parse::<u16>()
        {
            self.http.port = p;
        }
        self
    }

    /// Load configuration from environment variables only (no TOML file).
    pub fn load_env_only() -> Self {
        Self::load_from(std::path::Path::new(""))
    }
}

/// Translate a figment `Env` key into the matching dotted config path.
///
/// Required because the previous hand-rolled loader mapped `ZAKHOR_DB_PATH`
/// to `database.path`, `ZAKHOR_HTTP_HOST` to `http.host`, and
/// `ZAKHOR_HTTP_PORT` to `http.port`. Figment lowercases keys by default,
/// but we keep the mapping robust against any case variant.
pub(crate) fn map_env_key(key: &UncasedStr) -> Uncased<'_> {
    let mapped: Uncased<'_> = match key.as_str().to_ascii_uppercase().as_str() {
        "DB_PATH" => Uncased::new("database.path"),
        "HTTP_HOST" => Uncased::new("http.host"),
        "HTTP_PORT" => Uncased::new("http.port"),
        "MODELS_CACHE_DIR" => Uncased::new("models.cache_dir"),
        other => Uncased::new(other.to_ascii_lowercase()),
    };
    mapped
}
