use super::*;

#[test]
fn test_entity_iri() {
    let iri = entity_iri();
    assert!(iri.as_str().contains("zakhor"));
    assert!(iri.as_str().ends_with("Entity"));
}

#[test]
fn test_decision_iri() {
    let iri = decision_iri();
    assert!(iri.as_str().contains("zakhor"));
    assert!(iri.as_str().ends_with("Decision"));
}

#[test]
fn test_project_iri() {
    let iri = project_iri();
    assert!(iri.as_str().ends_with("Project"));
}

#[test]
fn test_issue_iri() {
    let iri = issue_iri();
    assert!(iri.as_str().ends_with("Issue"));
}

#[test]
fn test_constraint_iri() {
    let iri = constraint_iri();
    assert!(iri.as_str().ends_with("Constraint"));
}

#[test]
fn test_observation_iri() {
    let iri = observation_iri();
    assert!(iri.as_str().ends_with("Observation"));
}

#[test]
fn test_tool_call_iri() {
    let iri = tool_call_iri();
    assert!(iri.as_str().ends_with("ToolCall"));
}

#[test]
fn test_has_entity_iri() {
    let iri = has_entity_iri();
    assert!(iri.as_str().ends_with("hasEntity"));
}

#[test]
fn test_has_relation_iri() {
    let iri = has_relation_iri();
    assert!(iri.as_str().ends_with("hasRelation"));
}

#[test]
fn test_provenance_graph_iri() {
    let iri = provenance_graph_iri();
    assert!(iri.as_str().ends_with("provenanceGraph"));
}

#[test]
fn test_decision_context_iri() {
    let iri = decision_context_iri();
    assert!(iri.as_str().ends_with("decisionContext"));
}

#[test]
fn test_decision_outcome_iri() {
    let iri = decision_outcome_iri();
    assert!(iri.as_str().ends_with("decisionOutcome"));
}

#[test]
fn test_decision_alternative_iri() {
    let iri = decision_alternative_iri();
    assert!(iri.as_str().ends_with("alternative"));
}

#[test]
fn test_decision_rationale_iri() {
    let iri = decision_rationale_iri();
    assert!(iri.as_str().ends_with("decisionRationale"));
}

#[test]
fn test_decision_status_iri() {
    let iri = decision_status_iri();
    assert!(iri.as_str().ends_with("decisionStatus"));
}

#[test]
fn test_tool_name_iri() {
    let iri = tool_name_iri();
    assert!(iri.as_str().ends_with("toolName"));
}

#[test]
fn test_tool_arguments_iri() {
    let iri = tool_arguments_iri();
    assert!(iri.as_str().ends_with("toolArguments"));
}

#[test]
fn test_session_id_iri() {
    let iri = session_id_iri();
    assert!(iri.as_str().ends_with("sessionId"));
}

#[test]
fn test_timestamp_iri() {
    let iri = timestamp_iri();
    assert!(iri.as_str().ends_with("timestamp"));
}

#[test]
fn test_conflicts_with_iri() {
    let iri = conflicts_with_iri();
    assert!(iri.as_str().contains("zakhor"));
    assert!(iri.as_str().ends_with("conflictsWith"));
}

#[test]
fn test_depends_on_iri() {
    let iri = depends_on_iri();
    assert!(iri.as_str().contains("zakhor"));
    assert!(iri.as_str().ends_with("dependsOn"));
}

#[test]
fn test_supersedes_iri() {
    let iri = supersedes_iri();
    assert!(iri.as_str().contains("zakhor"));
    assert!(iri.as_str().ends_with("supersedes"));
}

#[test]
fn test_evidence_for_iri() {
    let iri = evidence_for_iri();
    assert!(iri.as_str().contains("zakhor"));
    assert!(iri.as_str().ends_with("evidenceFor"));
}

#[test]
fn test_belongs_to_project_iri() {
    let iri = belongs_to_project_iri();
    assert!(iri.as_str().ends_with("belongsToProject"));
}

#[test]
fn test_repository_iri() {
    let iri = repository_iri();
    assert!(iri.as_str().ends_with("Repository"));
}

#[test]
fn test_belongs_to_repository_iri() {
    let iri = belongs_to_repository_iri();
    assert!(iri.as_str().ends_with("belongsToRepository"));
}

#[test]
fn test_code_location_iri() {
    let iri = code_location_iri();
    assert!(iri.as_str().ends_with("codeLocation"));
}

#[test]
fn test_observation_content_iri() {
    let iri = observation_content_iri();
    assert!(iri.as_str().ends_with("observationContent"));
}

#[test]
fn test_observation_created_at_iri() {
    let iri = observation_created_at_iri();
    assert!(iri.as_str().ends_with("observationCreatedAt"));
}

#[test]
fn test_graph_importance_iri() {
    let iri = graph_importance_iri();
    assert!(iri.as_str().ends_with("graphImportance"));
}

#[test]
fn test_provenance_quality_iri() {
    let iri = provenance_quality_iri();
    assert!(iri.as_str().ends_with("provenanceQuality"));
}

#[test]
fn test_all_class_iris_use_zakhor_ns() {
    let iris = [
        entity_iri(),
        decision_iri(),
        project_iri(),
        repository_iri(),
        issue_iri(),
        constraint_iri(),
        observation_iri(),
        tool_call_iri(),
    ];
    for iri in &iris {
        assert!(
            iri.as_str().starts_with("http://zakhor/ns/"),
            "class IRI should start with zakhor ns: {}",
            iri
        );
    }
}

#[test]
fn test_all_predicate_iris_use_zakhor_ns() {
    let iris = [
        conflicts_with_iri(),
        depends_on_iri(),
        supersedes_iri(),
        evidence_for_iri(),
        belongs_to_project_iri(),
        belongs_to_repository_iri(),
        code_location_iri(),
        observation_content_iri(),
        observation_created_at_iri(),
        decision_status_iri(),
        graph_importance_iri(),
        provenance_quality_iri(),
        has_entity_iri(),
        has_relation_iri(),
        provenance_graph_iri(),
        decision_context_iri(),
        decision_outcome_iri(),
        decision_alternative_iri(),
        decision_rationale_iri(),
        tool_name_iri(),
        tool_arguments_iri(),
        session_id_iri(),
        timestamp_iri(),
    ];
    for iri in &iris {
        assert!(
            iri.as_str().starts_with("http://zakhor/ns/"),
            "predicate IRI should start with zakhor ns: {}",
            iri
        );
    }
}

#[test]
fn test_decision_status_constants() {
    assert_eq!(decision_status::ACTIVE, "active");
    assert_eq!(decision_status::SUPERSEDED, "superseded");
}

#[test]
fn test_named_graph_prefix() {
    assert_eq!(NAMED_GRAPH_PREFIX, "http://zakhor/ns/graph/");
}

macro_rules! assert_valid_uri_newtype {
    ($newtype:ty) => {
        for input in [
            "http://zakhor/ns/Entity",
            "http://example.com/e1",
            "urn:uuid:abc-123",
        ] {
            let parsed = <$newtype>::parse(input.to_owned());
            assert!(parsed.is_ok(), "expected {input:?} to parse");
            assert_eq!(parsed.expect("input was just checked").as_str(), input);
        }
    };
}

macro_rules! assert_invalid_uri_newtype {
    ($newtype:ty) => {
        for input in [
            "",
            "not-an-iri",
            "http://exa mple.com/x",
            "<http://example.com/x>",
        ] {
            assert!(
                <$newtype>::parse(input.to_owned()).is_err(),
                "expected {input:?} to be rejected"
            );
        }
    };
}

#[test]
fn entity_uri_parses_valid_instance_iris() {
    assert_valid_uri_newtype!(EntityUri);
}

#[test]
fn entity_uri_rejects_malformed_iris() {
    assert_invalid_uri_newtype!(EntityUri);
}

#[test]
fn observation_uri_parses_valid_instance_iris() {
    assert_valid_uri_newtype!(ObservationUri);
}

#[test]
fn observation_uri_rejects_malformed_iris() {
    assert_invalid_uri_newtype!(ObservationUri);
}

#[test]
fn project_uri_parses_valid_instance_iris() {
    assert_valid_uri_newtype!(ProjectUri);
}

#[test]
fn project_uri_rejects_malformed_iris() {
    assert_invalid_uri_newtype!(ProjectUri);
}

#[test]
fn repository_uri_parses_valid_instance_iris() {
    assert_valid_uri_newtype!(RepositoryUri);
}

#[test]
fn repository_uri_rejects_malformed_iris() {
    assert_invalid_uri_newtype!(RepositoryUri);
}

#[test]
fn decision_uri_parses_valid_instance_iris() {
    assert_valid_uri_newtype!(DecisionUri);
}

#[test]
fn decision_uri_rejects_malformed_iris() {
    assert_invalid_uri_newtype!(DecisionUri);
}

#[test]
fn relation_predicate_parses_valid_instance_iris() {
    assert_valid_uri_newtype!(RelationPredicate);
}

#[test]
fn relation_predicate_rejects_malformed_iris() {
    assert_invalid_uri_newtype!(RelationPredicate);
}

#[test]
fn entity_uri_round_trips_through_json() {
    let original =
        EntityUri::parse("http://example.com/e1".to_owned()).expect("valid IRI should parse");
    let json = serde_json::to_string(&original).expect("newtype should serialize");
    let decoded: EntityUri = serde_json::from_str(&json).expect("newtype should deserialize");

    assert_eq!(decoded, original);
}

#[test]
fn entity_uri_exposes_oxiri_for_storage() {
    let entity =
        EntityUri::parse("http://example.com/e1".to_owned()).expect("valid IRI should parse");
    let iri: &oxiri::Iri<String> = entity.as_iri();

    assert_eq!(iri.as_str(), "http://example.com/e1");
}

#[test]
fn entity_uri_serializes_as_a_bare_json_string() {
    // `#[serde(transparent)]` is what keeps MCP/REST payloads byte-identical after
    // the contract migration: a typed URI must still serialize as a plain string,
    // not as an object.
    let uri = EntityUri::parse("http://example.com/e1").expect("valid IRI");
    let json = serde_json::to_string(&uri).expect("serialize");

    assert_eq!(json, "\"http://example.com/e1\"");
}

// -- entity_mention_iri ------------------------------------------------------

#[test]
fn entity_mention_iri_keeps_simple_labels_readable() {
    assert_eq!(
        entity_mention_iri("Kubernetes"),
        "http://zakhor/ns/entity/Kubernetes"
    );
    assert_eq!(
        entity_mention_iri("Mountain View"),
        "http://zakhor/ns/entity/Mountain%20View"
    );
}

#[test]
fn entity_mention_iri_escapes_iri_illegal_characters() {
    // `/` would create a second path segment, `%` would fake an escape, `#`
    // would start a fragment, `"` and `<` are simply illegal in an IRI path.
    assert_eq!(
        entity_mention_iri("AC/DC"),
        "http://zakhor/ns/entity/AC%2FDC"
    );
    assert_eq!(
        entity_mention_iri("100% sure"),
        "http://zakhor/ns/entity/100%25%20sure"
    );
    assert_eq!(
        entity_mention_iri("AT&T <beta>"),
        "http://zakhor/ns/entity/AT&T%20%3Cbeta%3E"
    );
}

#[test]
fn entity_mention_iri_passes_non_ascii_through() {
    // RDF IRIs (RFC 3987) allow non-ASCII; keeping it unescaped keeps the
    // node readable in the graph.
    assert_eq!(
        entity_mention_iri("München"),
        "http://zakhor/ns/entity/München"
    );
}

#[test]
fn entity_mention_iri_is_always_a_valid_iri() {
    let samples = [
        "Steve Jobs",
        "R&D (EMEA)",
        "a-b_c.d~e",
        "quote\"name",
        "semi;colon:colon",
        "München // Büro",
        "100%",
        "tab\tsep",
    ];
    for label in samples {
        let iri = entity_mention_iri(label);
        // The same validation the SPARQL builder applies before persisting.
        let parsed = oxiri::Iri::parse(iri.clone());
        assert!(
            parsed.is_ok(),
            "entity_mention_iri({label:?}) = {iri:?} is not a valid IRI: {parsed:?}"
        );
    }
}

// -- relation_predicate_iri --------------------------------------------------

#[test]
fn relation_predicate_iri_uses_the_relation_namespace() {
    assert_eq!(
        relation_predicate_iri("worksAt"),
        "http://zakhor/ns/relation/worksAt"
    );
}

// -- canonical schema coverage ----------------------------------------------

#[test]
fn test_canonical_schema_covers_all_vocabulary_classes() {
    let expected = [
        ("Entity", entity_iri().as_str()),
        ("Decision", decision_iri().as_str()),
        ("Project", project_iri().as_str()),
        ("Repository", repository_iri().as_str()),
        ("Issue", issue_iri().as_str()),
        ("Constraint", constraint_iri().as_str()),
        ("Observation", observation_iri().as_str()),
        ("ToolCall", tool_call_iri().as_str()),
        ("CodeContainer", code_container_iri().as_str()),
        ("CodeSymbol", code_symbol_iri().as_str()),
    ];
    for (name, iri) in expected {
        assert!(
            CLASSES.iter().any(|c| c.name == name && c.iri() == iri),
            "canonical schema must declare class {name} at {iri}"
        );
    }
}

#[test]
fn test_canonical_schema_covers_all_vocabulary_properties() {
    let expected = [
        ("hasEntity", has_entity_iri().as_str()),
        ("hasRelation", has_relation_iri().as_str()),
        ("provenanceGraph", provenance_graph_iri().as_str()),
        ("decisionContext", decision_context_iri().as_str()),
        ("decisionOutcome", decision_outcome_iri().as_str()),
        ("alternative", decision_alternative_iri().as_str()),
        ("decisionRationale", decision_rationale_iri().as_str()),
        ("decisionStatus", decision_status_iri().as_str()),
        ("conflictsWith", conflicts_with_iri().as_str()),
        ("dependsOn", depends_on_iri().as_str()),
        ("supersedes", supersedes_iri().as_str()),
        ("evidenceFor", evidence_for_iri().as_str()),
        ("belongsToProject", belongs_to_project_iri().as_str()),
        ("belongsToRepository", belongs_to_repository_iri().as_str()),
        ("toolName", tool_name_iri().as_str()),
        ("toolArguments", tool_arguments_iri().as_str()),
        ("sessionId", session_id_iri().as_str()),
        ("timestamp", timestamp_iri().as_str()),
        ("graphImportance", graph_importance_iri().as_str()),
        ("provenanceQuality", provenance_quality_iri().as_str()),
        ("codeLocation", code_location_iri().as_str()),
        ("codeLanguage", code_language_iri().as_str()),
        ("codeSymbolKind", code_symbol_kind_iri().as_str()),
        ("codeLineStart", code_line_start_iri().as_str()),
        ("tombstone", tombstone_iri().as_str()),
    ];
    for (name, iri) in expected {
        assert!(
            PROPERTIES.iter().any(|p| p.name == name && p.iri() == iri),
            "canonical schema must declare property {name} at {iri}"
        );
    }
}
