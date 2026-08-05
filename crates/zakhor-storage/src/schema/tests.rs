use super::*;
use crate::sparql::Prefix;

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
fn test_construct_query_well_formed() {
    let q = ontology_construct_query();
    assert!(q.starts_with("PREFIX"), "should start with PREFIX");
    assert!(q.contains("CONSTRUCT {"), "should contain CONSTRUCT");
    assert!(q.contains("WHERE {"), "should contain WHERE");
    let opens = q.matches('{').count();
    let closes = q.matches('}').count();
    assert_eq!(
        opens, closes,
        "braces should be balanced: {} opens vs {} closes",
        opens, closes
    );
}

#[test]
fn test_insert_query_well_formed() {
    let q = ontology_insert_query();
    assert!(q.starts_with("PREFIX"), "should start with PREFIX");
    assert!(q.contains("INSERT DATA {"), "should contain INSERT DATA");
    assert!(!q.contains("CONSTRUCT"), "should NOT contain CONSTRUCT");
    assert!(
        q.contains("<http://zakhor/ns/Entity>"),
        "should reference Entity IRI"
    );
    assert!(
        q.contains("<http://zakhor/ns/Decision>"),
        "should reference Decision IRI"
    );
    assert!(
        q.contains("<http://zakhor/ns/Project>"),
        "should reference Project IRI"
    );
    assert!(
        q.contains("<http://zakhor/ns/Issue>"),
        "should reference Issue IRI"
    );
    assert!(
        q.contains("<http://zakhor/ns/Constraint>"),
        "should reference Constraint IRI"
    );
    assert!(
        q.contains("<http://zakhor/ns/Observation>"),
        "should reference Observation IRI"
    );
    assert!(
        q.contains("<http://zakhor/ns/hasEntity>"),
        "should reference hasEntity IRI"
    );
    assert!(
        q.contains("<http://zakhor/ns/hasRelation>"),
        "should reference hasRelation IRI"
    );
    assert!(
        q.contains("<http://zakhor/ns/provenanceGraph>"),
        "should reference provenanceGraph IRI"
    );
    assert!(
        q.contains("<http://zakhor/ns/decisionContext>"),
        "should reference decisionContext IRI"
    );
    assert!(
        q.contains("<http://zakhor/ns/decisionRationale>"),
        "should reference decisionRationale IRI"
    );
    let opens = q.matches('{').count();
    let closes = q.matches('}').count();
    assert_eq!(
        opens, closes,
        "braces should be balanced: {} opens vs {} closes",
        opens, closes
    );
}

#[test]
fn test_all_six_classes_defined() {
    let iris = [
        entity_iri(),
        decision_iri(),
        project_iri(),
        issue_iri(),
        constraint_iri(),
        observation_iri(),
    ];
    for iri in &iris {
        assert!(
            iri.as_str().starts_with(Prefix::ZAKHOR),
            "class IRI should start with zakhor ns: {}",
            iri
        );
    }
}

#[test]
fn test_all_five_properties_defined() {
    let iris = [
        has_entity_iri(),
        has_relation_iri(),
        provenance_graph_iri(),
        decision_context_iri(),
        decision_outcome_iri(),
        decision_alternative_iri(),
        decision_rationale_iri(),
    ];
    for iri in &iris {
        assert!(
            iri.as_str().starts_with(Prefix::ZAKHOR),
            "property IRI should start with zakhor ns: {}",
            iri
        );
    }
}

// -- ontology_file_content tests --------------------------------------------

#[test]
fn test_ontology_file_contains_prefixes() {
    let ttl = ontology_file_content();
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
    let ttl = ontology_file_content();
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
fn test_ontology_file_has_all_six_classes() {
    let ttl = ontology_file_content();
    for class in &[
        "Entity",
        "Decision",
        "Project",
        "Issue",
        "Constraint",
        "Observation",
    ] {
        let pattern = format!("zakhor:{} a rdfs:Class", class);
        assert!(
            ttl.contains(&pattern),
            "missing class definition for {}",
            class
        );
    }
}

#[test]
fn test_ontology_file_has_all_five_properties() {
    let ttl = ontology_file_content();
    for prop in &[
        "hasEntity",
        "hasRelation",
        "provenanceGraph",
        "decisionContext",
        "decisionRationale",
    ] {
        let pattern = format!("zakhor:{} a rdf:Property", prop);
        assert!(
            ttl.contains(&pattern),
            "missing property definition for {}",
            prop
        );
    }
}

#[test]
fn test_ontology_file_each_triple_terminated() {
    let ttl = ontology_file_content();
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
