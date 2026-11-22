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
            entity_labels: super::extraction_defaults::default_entity_labels(),
            relation_labels: super::extraction_defaults::default_relation_labels(),
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
    /// A missing TOML file yields the built-in defaults plus any `ZAKHOR_*`
    /// environment overrides. A present but malformed TOML file is a hard error
    /// ([`ConfigError::MalformedToml`]) — silent fallbacks to defaults are never
    /// acceptable for an explicitly configured file.
    ///
    /// Once the layered configuration is assembled, [`Config::validate`] runs and
    /// any misconfiguration is surfaced via [`ConfigError::Invalid`].
    pub fn load_from(path: &std::path::Path) -> Result<Self, ConfigError> {
        use figment::providers::{Env, Format, Serialized, Toml};

        // Fail loudly on a present-but-malformed TOML file; never fall back to
        // defaults for an explicitly configured path (issue #95).
        if !path.as_os_str().is_empty() && path.is_file() {
            let source = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            let _: toml::Table =
                toml::from_str(&source).map_err(|source| ConfigError::MalformedToml {
                    path: path.to_path_buf(),
                    source,
                })?;
        }

        let mut figment = Figment::new().merge(Serialized::defaults(Config::default()));

        // Only merge TOML layer if path is not empty
        if !path.as_os_str().is_empty() {
            figment = figment.merge(Toml::file(path));
        }

        let result: Result<Self, _> = figment
            .merge(Env::prefixed("ZAKHOR_").map(map_env_key))
            .extract();

        let config = result.map_err(|e| ConfigError::Load {
            path: path.to_path_buf(),
            source: Box::new(e),
        })?;

        config.validate()?;

        Ok(config)
    }

    /// Load configuration from `./zakhor.toml` (preserved for backward compatibility).
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from(std::path::Path::new("./zakhor.toml"))
    }

    /// Load configuration from environment variables only (no TOML file).
    pub fn load_env_only() -> Result<Self, ConfigError> {
        Self::load_from(std::path::Path::new(""))
    }
}

/// Translate a figment `Env` key into the matching dotted config path.
///
/// `ZAKHOR_` prefixed variables are stripped of the prefix by the figment
/// [`Env`](figment::providers::Env) provider before this mapping runs, so e.g.
/// `ZAKHOR_LLM_ENDPOINT` arrives as `LLM_ENDPOINT` and maps to `llm.endpoint`.
///
/// The four legacy top-level overrides keep their fully-qualified paths:
/// `DB_PATH` → `database.path`, `HTTP_HOST` → `http.host`,
/// `HTTP_PORT` → `http.port`, `MODELS_CACHE_DIR` → `models.cache_dir`.
///
/// Any other key maps onto a nested config path: the first underscore-separated
/// segment names the config section and the remainder names the (snake_case)
/// field, so underscores inside field names survive. Namespaced overrides
/// therefore land correctly, e.g. `ZAKHOR_LLM_CONFIDENCE_THRESHOLD` →
/// `llm.confidence_threshold`, `ZAKHOR_EXTRACTION_ENTITY_THRESHOLD` →
/// `extraction.entity_threshold`, and `ZAKHOR_ENTITY_RESOLUTION_ALIAS_THRESHOLD`
/// → `entity_resolution.alias_threshold` (multi-word sections keep their flat
/// TOML-style name) — instead of silently becoming unknown top-level keys
/// (issue #95).
pub(crate) fn map_env_key(key: &UncasedStr) -> Uncased<'_> {
    let raw = key.as_str().to_ascii_uppercase();
    let mapped: String = if raw == "DB_PATH" {
        "database.path".to_string()
    } else if raw == "HTTP_HOST" {
        "http.host".to_string()
    } else if raw == "HTTP_PORT" {
        "http.port".to_string()
    } else if raw == "MODELS_CACHE_DIR" {
        "models.cache_dir".to_string()
    } else {
        let lower = raw.to_ascii_lowercase();
        // Multi-word sections (e.g. `entity_resolution`) keep their flat name
        // as the section key; the remainder is the snake_case field.
        const MULTI_WORD_SECTIONS: [&str; 3] =
            ["entity_resolution", "code_indexing", "tool_capture"];
        let mut mapped = String::new();
        for section in MULTI_WORD_SECTIONS {
            let prefix = format!("{section}_");
            if let Some(field) = lower.strip_prefix(&prefix) {
                mapped = format!("{section}.{field}");
                break;
            }
        }
        if mapped.is_empty() {
            if let Some((section, field)) = lower.split_once('_') {
                mapped = format!("{section}.{field}");
            } else {
                mapped = lower;
            }
        }
        mapped
    };
    Uncased::new(mapped)
}
