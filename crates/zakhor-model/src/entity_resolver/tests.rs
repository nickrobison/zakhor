use zakhor_common::config::EntityResolutionConfig;

use super::*;

#[test]
fn test_new_resolver_empty() {
    let config = EntityResolutionConfig::default();
    let resolver = EntityResolver::new(config);
    let result = resolver.resolve("unknown entity");
    assert!(result.is_new);
    assert_eq!(result.resolution_tier, 0);
    assert_eq!(result.score, 0.0);
}

#[test]
fn test_tier1_alias_match() {
    let config = EntityResolutionConfig::default();
    let mut resolver = EntityResolver::new(config);
    resolver
        .register_alias("alice", "http://zakhor/ns/entity/alice")
        .expect("test URI should be valid");
    resolver
        .register_alias("bob", "http://zakhor/ns/entity/bob")
        .expect("test URI should be valid");

    let result = resolver.resolve("Alice"); // case-insensitive
    assert!(!result.is_new);
    assert_eq!(result.resolution_tier, 1);
    assert_eq!(
        result.resolved_uri.unwrap().as_str(),
        "http://zakhor/ns/entity/alice"
    );
}

#[test]
fn test_register_aliases_batch() {
    let config = EntityResolutionConfig::default();
    let mut resolver = EntityResolver::new(config);
    resolver
        .register_aliases(&[
            ("foo".to_string(), "http://zakhor/ns/entity/foo".to_string()),
            ("bar".to_string(), "http://zakhor/ns/entity/bar".to_string()),
        ])
        .expect("test URIs should be valid");

    assert_eq!(
        resolver.resolve("foo").resolved_uri.unwrap().as_str(),
        "http://zakhor/ns/entity/foo"
    );
    assert_eq!(
        resolver.resolve("Bar").resolved_uri.unwrap().as_str(),
        "http://zakhor/ns/entity/bar"
    );
}

#[test]
fn test_resolve_batch() {
    let config = EntityResolutionConfig::default();
    let mut resolver = EntityResolver::new(config);
    resolver
        .register_alias("known1", "http://zakhor/ns/entity/k1")
        .expect("test URI should be valid");
    resolver
        .register_alias("known2", "http://zakhor/ns/entity/k2")
        .expect("test URI should be valid");

    let labels = vec![
        "known1".to_string(),
        "unknown".to_string(),
        "known2".to_string(),
    ];
    let results = resolver.resolve_batch(&labels);
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].resolution_tier, 1);
    assert!(results[1].is_new);
    assert_eq!(results[2].resolution_tier, 1);
}

#[test]
fn test_default_config_thresholds() {
    let config = EntityResolutionConfig::default();
    assert_eq!(config.alias_threshold, 1.0);
    assert_eq!(config.tantivy_threshold, 0.85);
    assert_eq!(config.fastembed_threshold, 0.78);
}
