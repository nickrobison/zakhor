use super::types::*;
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
            config.extraction = file_config.extraction;
            config.embedding = file_config.embedding;
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
