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
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut config = Config::default();

        if let Ok(content) = std::fs::read_to_string("./zakhor.toml")
            && let Ok(file_config) = toml::from_str::<Config>(&content)
        {
            config.database.path = file_config.database.path;
            config.http.host.clone_from(&file_config.http.host);
            config.http.port = file_config.http.port;
            config.llm = file_config.llm;
            config.entity_resolution = file_config.entity_resolution;
            config.ranking = file_config.ranking;
            config.code_indexing = file_config.code_indexing;
            config.tool_capture = file_config.tool_capture;
            config.background = file_config.background;
        }

        if let Ok(path) = std::env::var("ZAKHOR_DB_PATH") {
            config.database.path = PathBuf::from(path);
        }
        if let Ok(host) = std::env::var("ZAKHOR_HTTP_HOST") {
            config.http.host = host;
        }
        if let Ok(port) = std::env::var("ZAKHOR_HTTP_PORT")
            && let Ok(p) = port.parse::<u16>()
        {
            config.http.port = p;
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.database.path, PathBuf::from("./zakhor-db"));
        assert_eq!(config.http.host, "127.0.0.1");
        assert_eq!(config.http.port, 3000);
        assert_eq!(config.llm.endpoint, "http://localhost:11434/api/generate");
        assert_eq!(config.llm.model, "llama3");
        assert_eq!(config.entity_resolution.alias_threshold, 1.0);
        assert_eq!(config.ranking.lexical_weight, 0.3);
        assert_eq!(config.code_indexing.max_parallel_parsers, 4);
        assert_eq!(config.tool_capture.max_evidence_per_decision, 50);
        assert_eq!(config.background.worker_count, 2);
    }

    #[test]
    fn test_toml_with_all_sections() {
        let toml_content = r#"
[database]
path = "/custom/db"

[http]
host = "0.0.0.0"
port = 8080

[llm]
endpoint = "http://ollama:11434/api/generate"
model = "llama3"
extraction_timeout_secs = 60
confidence_threshold = 0.8

[entity_resolution]
alias_threshold = 0.95
tantivy_threshold = 0.8
fastembed_threshold = 0.7

[ranking]
graph_importance_weight = 0.4
provenance_quality_weight = 0.1
lexical_weight = 0.3
semantic_weight = 0.2

[code_indexing]
max_parallel_parsers = 8
repo_poll_interval_secs = 600

[tool_capture]
max_evidence_per_decision = 100
session_timeout_minutes = 30

[background]
worker_count = 4
"#;
        let config: Config = toml::from_str(toml_content).expect("TOML should parse");
        assert_eq!(config.database.path, PathBuf::from("/custom/db"));
        assert_eq!(config.http.host, "0.0.0.0");
        assert_eq!(config.http.port, 8080);
        assert_eq!(config.llm.endpoint, "http://ollama:11434/api/generate");
        assert_eq!(config.llm.extraction_timeout_secs, 60);
        assert_eq!(config.entity_resolution.alias_threshold, 0.95);
        assert_eq!(config.entity_resolution.tantivy_threshold, 0.8);
        assert_eq!(config.entity_resolution.fastembed_threshold, 0.7);
        assert_eq!(config.ranking.graph_importance_weight, 0.4);
        assert_eq!(config.code_indexing.max_parallel_parsers, 8);
        assert_eq!(config.tool_capture.max_evidence_per_decision, 100);
        assert_eq!(config.background.worker_count, 4);
    }

    #[test]
    fn test_toml_without_new_sections() {
        let toml_content = r#"
[database]
path = "/custom/db"
"#;
        let config: Config =
            toml::from_str(toml_content).expect("TOML without new sections should parse");
        assert_eq!(config.database.path, PathBuf::from("/custom/db"));
        assert_eq!(config.llm.model, "llama3");
        assert_eq!(config.entity_resolution.alias_threshold, 1.0);
        assert_eq!(config.background.worker_count, 2);
    }
}
