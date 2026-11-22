// The layered config loader returns a composite error type; test helpers
// returning `Result` are intentionally small wrappers around it.
#![allow(clippy::result_large_err)]

use super::defaults::map_env_key;
use super::*;
use std::path::PathBuf;

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
    assert!(config.extraction.model_path.as_os_str().is_empty());
    // Zero-shot extraction needs a label set; an empty default made every
    // install without a hand-written zakhor.toml fail (issue #73).
    assert!(!config.extraction.entity_labels.is_empty());
    assert!(!config.extraction.relation_labels.is_empty());
    assert_eq!(config.extraction.entity_threshold, 0.5);
    assert_eq!(config.extraction.relation_threshold, 0.5);
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

[extraction]
model_path = "/home/user/models/gliner"
tokenizer_path = "/home/user/models/gliner-tokenizer"
entity_labels = ["Person", "Organization", "Location"]
relation_labels = ["works_at", "located_in"]
entity_threshold = 0.6
relation_threshold = 0.55
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
    assert_eq!(
        config.extraction.model_path,
        PathBuf::from("/home/user/models/gliner")
    );
    assert_eq!(
        config.extraction.tokenizer_path,
        PathBuf::from("/home/user/models/gliner-tokenizer")
    );
    assert_eq!(
        config.extraction.entity_labels,
        vec![
            "Person".to_string(),
            "Organization".to_string(),
            "Location".to_string()
        ]
    );
    assert_eq!(
        config.extraction.relation_labels,
        vec!["works_at".to_string(), "located_in".to_string()]
    );
    assert_eq!(config.extraction.entity_threshold, 0.6);
    assert_eq!(config.extraction.relation_threshold, 0.55);
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
    assert!(config.extraction.model_path.as_os_str().is_empty());
    // Zero-shot extraction needs a label set; an empty default made every
    // install without a hand-written zakhor.toml fail (issue #73).
    assert!(!config.extraction.entity_labels.is_empty());
    assert!(!config.extraction.relation_labels.is_empty());
    assert_eq!(config.extraction.entity_threshold, 0.5);
}

#[test]
fn test_load_from_reads_custom_path_toml() -> Result<(), ConfigError> {
    let dir = std::env::temp_dir().join(format!("zakhor-config-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let toml_path = dir.join("custom.toml");
    std::fs::write(
        &toml_path,
        r#"
[database]
path = "/from/toml"

[http]
host = "10.0.0.1"
port = 4242
"#,
    )
    .expect("write toml");

    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        let config = Config::load_from(&toml_path).expect("load");
        assert_eq!(config.database.path, PathBuf::from("/from/toml"));
        assert_eq!(config.http.host, "10.0.0.1");
        assert_eq!(config.http.port, 4242);
        assert_eq!(config.llm.model, "llama3");
        assert_eq!(config.background.worker_count, 2);
        Ok(())
    });
    Ok(())
}

#[test]
fn test_load_from_missing_file_yields_defaults() {
    let missing = std::env::temp_dir().join(format!(
        "zakhor-config-missing-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    assert!(!missing.exists());

    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        let config = Config::load_from(&missing).expect("missing file yields defaults");
        assert_eq!(config.database.path, PathBuf::from("./zakhor-db"));
        assert_eq!(config.http.host, "127.0.0.1");
        assert_eq!(config.http.port, 3000);
        Ok(())
    });
}

#[test]
fn test_load_from_malformed_toml_fails_loudly() {
    let dir = std::env::temp_dir().join(format!("zakhor-config-malformed-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let toml_path = dir.join("bad.toml");
    std::fs::write(&toml_path, "this is = not valid [toml").expect("write bad toml");

    let err = Config::load_from(&toml_path).expect_err("malformed TOML must fail loudly");
    assert!(
        matches!(err, ConfigError::MalformedToml { .. }),
        "expected MalformedToml, got {err:?}"
    );
    let message = err.to_string();
    assert!(
        message.contains("malformed TOML") && message.contains(&toml_path.display().to_string()),
        "error should name the file and problem, got: {message}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_load_from_malformed_toml_in_parent_dir_fails_loudly() -> Result<(), ConfigError> {
    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        // Place a malformed config in the parent (jail root), then search from
        // a child directory using a relative path - matching figment's
        // parent-directory search behavior.
        jail.create_file("zakhor.toml", "this is = not valid [toml")?;
        let child = jail.create_dir("child")?;
        jail.change_dir(child)?;

        let err = Config::load_from(std::path::Path::new("zakhor.toml"))
            .expect_err("malformed TOML in parent dir must fail loudly");
        assert!(
            matches!(err, ConfigError::MalformedToml { .. }),
            "expected MalformedToml, got {err:?}"
        );
        let message = err.to_string();
        assert!(
            message.contains("malformed TOML") && message.contains("zakhor.toml"),
            "error should name the file and problem, got: {message}"
        );
        Ok(())
    });
    Ok(())
}

#[test]
fn test_models_cache_dir_from_toml() {
    let toml_content = r#"
[database]
path = "/tmp/db"

[models]
cache_dir = "/tmp/m"
"#;
    let config: Config = toml::from_str(toml_content).expect("TOML should parse");
    assert_eq!(config.models.cache_dir, Some(PathBuf::from("/tmp/m")));
}

#[test]
fn test_models_cache_dir_absent_in_toml() {
    let toml_content = r#"
[database]
path = "/custom/db"
"#;
    let config: Config =
        toml::from_str(toml_content).expect("TOML without new sections should parse");
    assert!(config.models.cache_dir.is_none());
}

#[test]
fn test_default_config_models_cache_dir_is_none() {
    let config = Config::default();
    assert!(config.models.cache_dir.is_none());
}

#[test]
fn test_map_env_key_models_cache_dir() {
    let u = figment::value::Uncased::new("MODELS_CACHE_DIR");
    assert_eq!(map_env_key(&u).as_str(), "models.cache_dir");
}

#[test]
fn test_load_from_round_trips_models_cache_dir() {
    let dir = std::env::temp_dir().join(format!("zakhor-config-models-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let toml_path = dir.join("models.toml");
    std::fs::write(
        &toml_path,
        r#"
[models]
cache_dir = "/tmp/models"
"#,
    )
    .expect("write toml");

    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        let config = Config::load_from(&toml_path).expect("valid TOML loads");
        assert_eq!(config.models.cache_dir, Some(PathBuf::from("/tmp/models")));
        Ok(())
    });

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_load_env_only_models_cache_dir_is_none() -> Result<(), ConfigError> {
    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        let config = Config::load_env_only().expect("config should load");
        assert!(config.models.cache_dir.is_none());
        Ok(())
    });
    Ok(())
}

#[test]
fn test_map_env_key_namespaced_keys() {
    let cases = [
        ("LLM_ENDPOINT", "llm.endpoint"),
        ("LLM_MODEL", "llm.model"),
        ("LLM_CONFIDENCE_THRESHOLD", "llm.confidence_threshold"),
        ("EXTRACTION_ENTITY_THRESHOLD", "extraction.entity_threshold"),
        (
            "EXTRACTION_RELATION_THRESHOLD",
            "extraction.relation_threshold",
        ),
        ("RANKING_SEMANTIC_WEIGHT", "ranking.semantic_weight"),
        ("BACKGROUND_WORKER_COUNT", "background.worker_count"),
        (
            "ENTITY_RESOLUTION_ALIAS_THRESHOLD",
            "entity_resolution.alias_threshold",
        ),
        ("EMBEDDING_ENABLED", "embedding.enabled"),
    ];
    for (raw, expected) in cases {
        let u = figment::value::Uncased::new(raw);
        assert_eq!(
            map_env_key(&u).as_str(),
            expected,
            "unexpected mapping for ZAKHOR_{raw}"
        );
    }
}

#[test]
fn test_env_overrides_namespaced_keys_reach_nested_config() -> Result<(), ConfigError> {
    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        jail.set_env("ZAKHOR_LLM_ENDPOINT", "http://ollama:11434/api/generate");
        jail.set_env("ZAKHOR_LLM_MODEL", "llama3:latest");
        jail.set_env("ZAKHOR_LLM_CONFIDENCE_THRESHOLD", "0.9");
        jail.set_env("ZAKHOR_EXTRACTION_ENTITY_THRESHOLD", "0.42");
        jail.set_env("ZAKHOR_RANKING_SEMANTIC_WEIGHT", "0.35");
        jail.set_env("ZAKHOR_BACKGROUND_WORKER_COUNT", "3");
        jail.set_env("ZAKHOR_EMBEDDING_ENABLED", "true");
        jail.set_env("ZAKHOR_MODELS_CACHE_DIR", "/tmp/zakhor-models");
        let config = Config::load_env_only().expect("config should load");
        assert_eq!(config.llm.endpoint, "http://ollama:11434/api/generate");
        assert_eq!(config.llm.model, "llama3:latest");
        assert_eq!(config.llm.confidence_threshold, 0.9);
        assert_eq!(config.extraction.entity_threshold, 0.42);
        assert_eq!(config.ranking.semantic_weight, 0.35);
        assert_eq!(config.background.worker_count, 3);
        assert!(config.embedding.enabled);
        assert_eq!(
            config.models.cache_dir,
            Some(PathBuf::from("/tmp/zakhor-models"))
        );
        Ok(())
    });
    Ok(())
}

#[test]
fn test_validate_accepts_default_config() {
    Config::default()
        .validate()
        .expect("built-in defaults must pass validation");
}

#[test]
fn test_validate_rejects_out_of_range_values() {
    let mut config = Config::default();
    config.llm.confidence_threshold = 1.5;
    let err = config
        .validate()
        .expect_err("confidence_threshold > 1 must be rejected");
    assert!(
        matches!(err, ConfigError::Invalid { .. }),
        "expected Invalid, got {err:?}"
    );
    assert!(err.to_string().contains("llm.confidence_threshold"));

    let mut config = Config::default();
    config.ranking.lexical_weight = -0.1;
    let err = config
        .validate()
        .expect_err("ranking weight < 0 must be rejected");
    assert!(err.to_string().contains("ranking.lexical_weight"));

    let mut config = Config::default();
    config.extraction.relation_threshold = f32::NAN;
    let err = config
        .validate()
        .expect_err("NaN threshold must be rejected");
    assert!(err.to_string().contains("extraction.relation_threshold"));

    let mut config = Config::default();
    config.http.port = 0;
    let err = config.validate().expect_err("port 0 must be rejected");
    assert!(err.to_string().contains("http.port"));

    let mut config = Config::default();
    config.background.worker_count = 0;
    let err = config
        .validate()
        .expect_err("worker_count 0 must be rejected");
    assert!(err.to_string().contains("background.worker_count"));

    let mut config = Config::default();
    config.http.host = "   ".to_string();
    let err = config
        .validate()
        .expect_err("empty http.host must be rejected");
    assert!(err.to_string().contains("http.host"));
}

#[test]
fn test_load_from_rejects_invalid_values() {
    let dir = std::env::temp_dir().join(format!("zakhor-config-invalid-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let toml_path = dir.join("invalid.toml");
    std::fs::write(
        &toml_path,
        r#"
[background]
worker_count = 0
"#,
    )
    .expect("write toml");

    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        let err = Config::load_from(&toml_path).expect_err("invalid worker_count must fail loudly");
        assert!(
            matches!(err, ConfigError::Invalid { .. }),
            "expected Invalid, got {err:?}"
        );
        let message = err.to_string();
        assert!(
            message.contains("background.worker_count") && message.contains("0"),
            "error should name the offending key/value, got: {message}"
        );
        Ok(())
    });

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_load_from_rejects_invalid_host_and_probabilities() {
    let dir =
        std::env::temp_dir().join(format!("zakhor-config-invalid-host-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");

    let host_path = dir.join("bad-host.toml");
    std::fs::write(
        &host_path,
        r#"
[http]
host = "   "
"#,
    )
    .expect("write toml");
    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        let err = Config::load_from(&host_path).expect_err("empty http.host must fail");
        assert!(matches!(err, ConfigError::Invalid { .. }));
        assert!(err.to_string().contains("http.host"));
        Ok(())
    });

    let prob_path = dir.join("bad-prob.toml");
    std::fs::write(
        &prob_path,
        r#"
[llm]
confidence_threshold = -0.1
"#,
    )
    .expect("write toml");
    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        let err = Config::load_from(&prob_path).expect_err("negative probability must fail loudly");
        assert!(matches!(err, ConfigError::Invalid { .. }));
        assert!(err.to_string().contains("llm.confidence_threshold"));
        Ok(())
    });

    let port_path = dir.join("bad-port.toml");
    std::fs::write(
        &port_path,
        r#"
[http]
port = 0
"#,
    )
    .expect("write toml");
    figment::Jail::expect_with(|jail| {
        jail.clear_env();
        let err = Config::load_from(&port_path).expect_err("port 0 must fail loudly");
        assert!(matches!(err, ConfigError::Invalid { .. }));
        assert!(err.to_string().contains("http.port"));
        Ok(())
    });

    let _ = std::fs::remove_dir_all(&dir);
}
