use super::*;
use crate::sparql::Prefix;
use zakhor_common::config::extraction_defaults::default_relation_labels;
use zakhor_common::vocab::schema::{CLASSES, PROPERTIES};

/// The relation labels the ontology-content tests declare; the app defaults,
/// so the generated file exercises the relation properties.
fn test_relation_labels() -> Vec<String> {
    default_relation_labels()
}

#[test]
fn test_entity_iri_contains_zakhor() {
    let iri = entity_iri();
    assert!(
        iri.as_str().contains("zakhor"),
        "Entity IRI should contain zakhor"
    );
    assert!(
        iri.as_str().contains("Entity"),
        "Entity IRI should contain Entity"
    );
}

#[test]
fn test_decision_iri_contains_zakhor() {
    let iri = decision_iri();
    assert!(iri.as_str().contains("zakhor"));
    assert!(iri.as_str().ends_with("Decision"));
}

#[test]
fn test_has_entity_iri_contains_zakhor() {
    let iri = has_entity_iri();
    assert!(iri.as_str().contains("zakhor"));
    assert!(iri.as_str().ends_with("hasEntity"));
}

#[test]
fn test_all_schema_classes_use_zakhor_ns() {
    for class in CLASSES {
        assert!(
            class.iri().starts_with(Prefix::ZAKHOR),
            "class IRI should start with zakhor ns: {}",
            class.iri()
        );
    }
}

#[test]
fn test_all_schema_properties_use_zakhor_ns() {
    for prop in PROPERTIES {
        assert!(
            prop.iri().starts_with(Prefix::ZAKHOR),
            "property IRI should start with zakhor ns: {}",
            prop.iri()
        );
    }
}

// -- ontology_file_content tests --------------------------------------------

#[test]
fn test_ontology_file_contains_prefixes() {
    let ttl = ontology_file_content(&test_relation_labels());
    assert!(ttl.contains("@prefix rdf:"), "missing rdf prefix");
    assert!(ttl.contains("@prefix rdfs:"), "missing rdfs prefix");
    assert!(ttl.contains("@prefix xsd:"), "missing xsd prefix");
    assert!(ttl.contains("@prefix nrl:"), "missing nrl prefix");
    assert!(ttl.contains("@prefix zakhor:"), "missing zakhor prefix");
    assert!(
        ttl.contains("<http://zakhor/ns/>"),
        "zakhor namespace should match Prefix::ZAKHOR"
    );
}

#[test]
fn test_ontology_file_declares_nrl_ontology() {
    let ttl = ontology_file_content(&test_relation_labels());
    assert!(
        ttl.contains("a nrl:Namespace, nrl:Ontology"),
        "should declare zakhor as nrl:Namespace, nrl:Ontology"
    );
    assert!(
        ttl.contains("nrl:prefix \"zakhor\""),
        "should have nrl:prefix property"
    );
    assert!(
        ttl.contains("nrl:lastModified"),
        "should have nrl:lastModified property"
    );
}

#[test]
fn test_ontology_file_has_all_canonical_classes() {
    let ttl = ontology_file_content(&test_relation_labels());
    for class in CLASSES {
        let pattern = format!("zakhor:{} a rdfs:Class", class.name);
        assert!(
            ttl.contains(&pattern),
            "missing class definition for {}",
            class.name
        );
    }
}

#[test]
fn test_ontology_file_has_all_canonical_properties() {
    let ttl = ontology_file_content(&test_relation_labels());
    for prop in PROPERTIES {
        let pattern = format!("zakhor:{} a rdf:Property", prop.name);
        assert!(
            ttl.contains(&pattern),
            "missing property definition for {}",
            prop.name
        );
        assert!(
            ttl.contains(&format!("rdfs:domain {}", prop.domain)),
            "missing domain for {} (expected {})",
            prop.name,
            prop.domain
        );
        assert!(
            ttl.contains(&format!("rdfs:range {}", prop.range)),
            "missing range for {} (expected {})",
            prop.name,
            prop.range
        );
    }
}

#[test]
fn test_ontology_file_declares_relation_predicates() {
    let labels = test_relation_labels();
    let ttl = ontology_file_content(&labels);
    // Extracted relations are persisted as subject/predicate/object triples
    // whose predicate must be a declared property, so every configured relation
    // label needs its `ns/relation/{label}` property in the ontology. This is
    // the coupling `extract_and_store` depends on to persist what the model
    // predicts. The properties live under the dedicated `zrel:` prefix because
    // Tracker resolves property IRIs against declared prefixes only.
    for label in &labels {
        let pattern = format!("zrel:{label} a rdf:Property");
        assert!(
            ttl.contains(&pattern),
            "missing relation property definition for {label}"
        );
    }
    assert!(
        ttl.contains("@prefix zrel: <http://zakhor/ns/relation/> ."),
        "the relation namespace prefix must be declared"
    );

    // Labels that were never configured must not appear.
    assert!(
        !ttl.contains("zrel:customUnconfigured a rdf:Property"),
        "undeclared labels must not gain properties"
    );
}

#[test]
fn test_ontology_file_last_modified_is_derived_from_schema() {
    let labels = test_relation_labels();
    let ttl = ontology_file_content(&labels);
    // The timestamp must be derived from the canonical schema hash, not a
    // hardcoded literal that can silently go stale.
    let expected = zakhor_common::vocab::schema::ontology_last_modified(&labels);
    let pattern = format!("nrl:lastModified \"{expected}\"^^xsd:dateTime");
    assert!(
        ttl.contains(&pattern),
        "nrl:lastModified should be derived: expected {pattern:?}, got ontology file"
    );
}

#[test]
fn test_ontology_file_each_triple_terminated() {
    let ttl = ontology_file_content(&test_relation_labels());
    // Every non-empty, non-prefix line should end with '.', ';', or ',' (Turtle syntax).
    for (i, line) in ttl.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("@prefix") {
            continue;
        }
        let last = trimmed.chars().last().expect("line should not be empty");
        assert!(
            last == '.' || last == ';' || last == ',',
            "line {} ends with unexpected char {:?}: {:?}",
            i + 1,
            last,
            trimmed
        );
    }
}

#[test]
fn test_nrl_constant_correct() {
    assert_eq!(NRL, "http://tracker.api.gnome.org/ontology/v3/nrl#");
}
