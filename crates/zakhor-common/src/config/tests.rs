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
    assert!(config.extraction.entity_labels.is_empty());
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
    assert!(config.extraction.entity_labels.is_empty());
    assert_eq!(config.extraction.entity_threshold, 0.5);
}

#[test]
fn test_load_from_reads_custom_path_toml() {
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

    let config = Config::load_from(&toml_path);
    assert_eq!(config.database.path, PathBuf::from("/from/toml"));
    assert_eq!(config.http.host, "10.0.0.1");
    assert_eq!(config.http.port, 4242);
    assert_eq!(config.llm.model, "llama3");
    assert_eq!(config.background.worker_count, 2);

    let _ = std::fs::remove_dir_all(&dir);
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

    let config = Config::load_from(&missing);
    assert_eq!(config.database.path, PathBuf::from("./zakhor-db"));
    assert_eq!(config.http.host, "127.0.0.1");
    assert_eq!(config.http.port, 3000);
}

#[test]
fn test_load_from_malformed_toml_falls_back() {
    let dir = std::env::temp_dir().join(format!("zakhor-config-malformed-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let toml_path = dir.join("bad.toml");
    std::fs::write(&toml_path, "this is = not valid [toml").expect("write bad toml");

    let config = Config::load_from(&toml_path);
    assert_eq!(config.database.path, PathBuf::from("./zakhor-db"));
    assert_eq!(config.http.port, 3000);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_map_env_key_translates_known_vars() {
    let cases: &[(&str, &str)] = &[
        ("DB_PATH", "database.path"),
        ("db_path", "database.path"),
        ("HTTP_HOST", "http.host"),
        ("http_host", "http.host"),
        ("HTTP_PORT", "http.port"),
        ("http_port", "http.port"),
        ("FOO_BAR", "foo_bar"),
        ("foo_bar", "foo_bar"),
    ];
    for (input, expected) in cases {
        let u = figment::value::Uncased::new(*input);
        assert_eq!(map_env_key(&u).as_str(), *expected, "input={input}");
    }
}
