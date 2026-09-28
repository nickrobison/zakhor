//! Default label sets for the GLiNER extraction pipeline.
//!
//! GLiNER is zero-shot: the entity and relation labels are part of the prompt,
//! so an empty set leaves the model nothing to look for. Defaulting to empty
//! therefore made every install that did not hand-write a `zakhor.toml` fail
//! with `empty texts and/or entities` — an error describing the symptom rather
//! than the cause.
//!
//! These live in `zakhor-common` rather than `zakhor-model` because the
//! user-facing TOML config and the runtime extraction config are separate
//! types. Defining the lists once keeps the two from drifting, which is the
//! same failure mode `decision_text` was extracted to prevent.

/// Entity labels used when none are configured.
pub const DEFAULT_ENTITY_LABELS: &[&str] = &[
    "Person",
    "Organization",
    "Location",
    "Technology",
    "Event",
    "Product",
];

/// Relation labels used when none are configured.
pub const DEFAULT_RELATION_LABELS: &[&str] = &[
    "worksAt",
    "locatedIn",
    "uses",
    "founded",
    "partOf",
    "mentions",
];

/// Own the defaults as a `Vec<String>` for a config field.
pub fn default_entity_labels() -> Vec<String> {
    DEFAULT_ENTITY_LABELS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Own the defaults as a `Vec<String>` for a config field.
pub fn default_relation_labels() -> Vec<String> {
    DEFAULT_RELATION_LABELS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_default_label_sets_are_usable() {
        assert!(!DEFAULT_ENTITY_LABELS.is_empty());
        assert!(!DEFAULT_RELATION_LABELS.is_empty());
    }

    /// A blank or duplicated label silently wastes prompt budget or makes two
    /// configured labels indistinguishable.
    #[test]
    fn test_default_labels_are_clean() {
        for set in [DEFAULT_ENTITY_LABELS, DEFAULT_RELATION_LABELS] {
            let mut seen = HashSet::new();
            for label in set {
                assert!(!label.trim().is_empty(), "blank label in {set:?}");
                assert!(seen.insert(*label), "duplicate label {label:?} in {set:?}");
            }
        }
    }

    #[test]
    fn test_vec_conversions_match_the_constants() {
        assert_eq!(default_entity_labels().len(), DEFAULT_ENTITY_LABELS.len());
        assert_eq!(
            default_relation_labels().len(),
            DEFAULT_RELATION_LABELS.len()
        );
        assert_eq!(default_entity_labels()[0], DEFAULT_ENTITY_LABELS[0]);
    }
}
