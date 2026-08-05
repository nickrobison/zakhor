use oxiri::Iri;
use zakhor_common::vocab;
use zakhor_storage::sparql::Prefix;

use super::*;

fn iri(value: &str) -> Iri<String> {
    Iri::parse(value.to_owned()).expect("test IRIs should be valid")
}

#[test]
fn test_create_decision_args_struct() {
    let args = CreateDecisionArgs {
        context: "Test context".into(),
        outcome: "Approved".into(),
        alternatives: vec!["Alt A".into(), "Alt B".into()],
        rationale: "Because".into(),
        affects: vec![iri("http://zakhor/ns/entity/e1")],
        derived_from: vec![],
        supersedes: None,
        conflicts_with: vec![],
        depends_on: vec![],
        project_uri: None,
    };
    assert_eq!(args.context, "Test context");
    assert_eq!(args.alternatives.len(), 2);
}

#[test]
fn test_build_create_decision_sparql_basic() {
    let args = CreateDecisionArgs {
        context: "Context".into(),
        outcome: "Outcome".into(),
        alternatives: vec!["Alternative 1".into()],
        rationale: "Rationale".into(),
        affects: vec![],
        derived_from: vec![],
        supersedes: None,
        conflicts_with: vec![],
        depends_on: vec![],
        project_uri: None,
    };
    let sparql = build_create_decision_sparql(&args, &iri("http://zakhor/ns/decision/test-1"));
    assert!(sparql.contains("INSERT DATA"));
    assert!(sparql.contains("rdf:type"));
    assert!(sparql.contains("decisionContext"));
    assert!(sparql.contains("decisionOutcome"));
    assert!(sparql.contains("decisionRationale"));
    assert!(sparql.contains("decisionStatus"));
    assert!(sparql.contains("active"));
    assert!(sparql.contains("Alternative 1"));
}

#[test]
fn test_build_create_decision_with_relations() {
    let args = CreateDecisionArgs {
        context: "Ctx".into(),
        outcome: "Out".into(),
        alternatives: vec![],
        rationale: "Rat".into(),
        affects: vec![iri("http://zakhor/ns/entity/e1")],
        derived_from: vec![iri("urn:uuid:obs-1")],
        supersedes: Some(iri("http://zakhor/ns/decision/old")),
        conflicts_with: vec![iri("http://zakhor/ns/decision/conflict")],
        depends_on: vec![iri("http://zakhor/ns/decision/dep")],
        project_uri: Some(iri("http://zakhor/ns/project/p1")),
    };
    let sparql = build_create_decision_sparql(&args, &iri("http://zakhor/ns/decision/test-2"));
    assert!(sparql.contains("supersedes"));
    assert!(sparql.contains("conflictsWith"));
    assert!(sparql.contains("dependsOn"));
    assert!(sparql.contains("belongsToProject"));
    assert!(sparql.contains(Prefix::PROV_WAS_DERIVED_FROM));
    assert!(sparql.contains("provenanceGraph"));
}

#[test]
fn test_decision_status_constants() {
    assert_eq!(vocab::decision_status::ACTIVE, "active");
    assert_eq!(vocab::decision_status::SUPERSEDED, "superseded");
}
