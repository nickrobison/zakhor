#![allow(dead_code)]

use oxiri::Iri;
use std::collections::HashMap;
use std::sync::Mutex;
use zakhor_common::config::EntityResolutionConfig;
use zakhor_search::LexicalIndex;
use zakhor_search::SemanticIndex;

/// Result of a single entity resolution attempt.
#[derive(Clone, Debug)]
pub struct ResolvedEntity {
    /// The original extracted entity URI/label.
    pub extracted_label: String,
    /// The resolved (canonical) URI, or None if no match found.
    pub resolved_uri: Option<Iri<String>>,
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
    aliases: HashMap<String, Iri<String>>,
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
        let uri = Iri::parse(uri.to_string())
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
            if let Some(top) = results.first()
                && top.score >= self.config.tantivy_threshold as f64
            {
                match Iri::parse(top.id.as_str().to_owned()) {
                    Ok(uri) => {
                        return ResolvedEntity {
                            extracted_label: label.to_string(),
                            resolved_uri: Some(uri),
                            resolution_tier: 2,
                            score: top.score as f32,
                            is_new: false,
                        };
                    }
                    Err(e) => tracing::warn!(
                        entity_label = %label,
                        candidate_uri = %top.id,
                        error = %e,
                        "Ignoring invalid lexical candidate URI"
                    ),
                }
            }
        }

        // Tier 3: fastembed semantic search
        if let Some(ref semantic) = self.semantic
            && let Ok(mut sem) = semantic.lock()
        {
            let results = sem.search(label, 5);
            if let Some(top) = results.first()
                && top.score >= self.config.fastembed_threshold as f64
            {
                match Iri::parse(top.id.as_str().to_owned()) {
                    Ok(uri) => {
                        return ResolvedEntity {
                            extracted_label: label.to_string(),
                            resolved_uri: Some(uri),
                            resolution_tier: 3,
                            score: top.score as f32,
                            is_new: false,
                        };
                    }
                    Err(e) => tracing::warn!(
                        entity_label = %label,
                        candidate_uri = %top.id,
                        error = %e,
                        "Ignoring invalid semantic candidate URI"
                    ),
                }
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
