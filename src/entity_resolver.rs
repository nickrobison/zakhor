#![allow(dead_code)]

use crate::config::EntityResolutionConfig;
use crate::lexical::LexicalIndex;
use crate::semantic::SemanticIndex;
use iref::IriBuf;
use std::collections::HashMap;
use std::sync::Mutex;

/// Result of a single entity resolution attempt.
#[derive(Clone, Debug)]
pub struct ResolvedEntity {
    /// The original extracted entity URI/label.
    pub extracted_label: String,
    /// The resolved (canonical) URI, or None if no match found.
    pub resolved_uri: Option<IriBuf>,
    /// The tier that resolved this entity (1=alias, 2=tantivy, 3=fastembed, 0=unresolved).
    pub resolution_tier: u8,
    /// Similarity score of the match.
    pub score: f32,
    /// True if this is a newly created entity (no existing match).
    pub is_new: bool,
}

/// 3-tier entity resolution pipeline.
///
/// Tier 1: Exact alias match against known entity URIs.
/// Tier 2: Tantivy lexical search on entity labels.
/// Tier 3: fastembed semantic similarity on entity labels.
///
/// Resolution stops at the first tier where the top match exceeds the
/// configured threshold.
pub struct EntityResolver {
    config: EntityResolutionConfig,
    /// Known entity aliases: label -> URI
    aliases: HashMap<String, IriBuf>,
    /// Tantivy index for lexical search
    lexical: Option<LexicalIndex>,
    /// fastembed index for semantic search
    semantic: Option<Mutex<SemanticIndex>>,
}

impl EntityResolver {
    pub fn new(config: EntityResolutionConfig) -> Self {
        Self {
            config,
            aliases: HashMap::new(),
            lexical: None,
            semantic: None,
        }
    }

    /// Register an alias for exact-match resolution (Tier 1).
    pub fn register_alias(&mut self, label: &str, uri: &str) -> Result<(), String> {
        let uri = IriBuf::new(uri.to_string())
            .map_err(|e| format!("invalid URI for alias '{label}': {e}"))?;
        self.aliases.insert(label.to_lowercase(), uri);
        Ok(())
    }

    /// Register multiple aliases at once.
    pub fn register_aliases(&mut self, pairs: &[(String, String)]) -> Result<(), String> {
        for (label, uri) in pairs {
            self.register_alias(label, uri)?;
        }
        Ok(())
    }

    /// Attach a Tantivy lexical index for Tier 2 resolution.
    pub fn with_lexical(mut self, index: LexicalIndex) -> Self {
        self.lexical = Some(index);
        self
    }

    /// Attach a fastembed semantic index for Tier 3 resolution.
    pub fn with_semantic(mut self, index: Mutex<SemanticIndex>) -> Self {
        self.semantic = Some(index);
        self
    }

    /// Resolve a single extracted entity label to a canonical URI.
    ///
    /// Returns the resolution result with the tier and score.
    pub fn resolve(&self, label: &str) -> ResolvedEntity {
        let lower = label.to_lowercase();

        // Tier 1: Exact alias match
        if let Some(uri) = self.aliases.get(&lower) {
            return ResolvedEntity {
                extracted_label: label.to_string(),
                resolved_uri: Some(uri.clone()),
                resolution_tier: 1,
                score: 1.0,
                is_new: false,
            };
        }

        // Tier 2: Tantivy lexical search
        if let Some(ref lexical) = self.lexical {
            let results = lexical.search(label, 5).unwrap_or_default();
            if let Some(top) = results.into_iter().next()
                && top.score >= self.config.tantivy_threshold as f64
                && let Ok(uri) = IriBuf::new(top.id)
            {
                return ResolvedEntity {
                    extracted_label: label.to_string(),
                    resolved_uri: Some(uri),
                    resolution_tier: 2,
                    score: top.score as f32,
                    is_new: false,
                };
            }
        }

        // Tier 3: fastembed semantic search
        if let Some(ref semantic) = self.semantic
            && let Ok(mut sem) = semantic.lock()
        {
            let results = sem.search(label, 5);
            if let Some(top) = results.into_iter().next()
                && top.score >= self.config.fastembed_threshold as f64
                && let Ok(uri) = IriBuf::new(top.id)
            {
                return ResolvedEntity {
                    extracted_label: label.to_string(),
                    resolved_uri: Some(uri),
                    resolution_tier: 3,
                    score: top.score as f32,
                    is_new: false,
                };
            }
        }

        // Unresolved: entity is new
        ResolvedEntity {
            extracted_label: label.to_string(),
            resolved_uri: None,
            resolution_tier: 0,
            score: 0.0,
            is_new: true,
        }
    }

    /// Resolve multiple entity labels in batch.
    pub fn resolve_batch(&self, labels: &[String]) -> Vec<ResolvedEntity> {
        labels.iter().map(|l| self.resolve(l)).collect()
    }
}

#[cfg(test)]
mod tests {
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
}
