#![allow(dead_code)]

use iref::Iri;
use static_iref::iri;

// --- Class IRIs ---

pub fn entity_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Entity")
}

pub fn decision_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Decision")
}

pub fn project_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Project")
}

pub fn issue_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Issue")
}

pub fn constraint_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Constraint")
}

pub fn observation_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Observation")
}

pub fn tool_call_iri() -> &'static Iri {
    iri!("http://zakhor/ns/ToolCall")
}

// --- Predicate IRIs ---

pub fn has_entity_iri() -> &'static Iri {
    iri!("http://zakhor/ns/hasEntity")
}

pub fn has_relation_iri() -> &'static Iri {
    iri!("http://zakhor/ns/hasRelation")
}

pub fn provenance_graph_iri() -> &'static Iri {
    iri!("http://zakhor/ns/provenanceGraph")
}

pub fn decision_context_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionContext")
}

pub fn decision_outcome_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionOutcome")
}

pub fn decision_alternative_iri() -> &'static Iri {
    iri!("http://zakhor/ns/alternative")
}

pub fn decision_rationale_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionRationale")
}

pub fn decision_status_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionStatus")
}

pub fn conflicts_with_iri() -> &'static Iri {
    iri!("http://zakhor/ns/conflictsWith")
}

pub fn depends_on_iri() -> &'static Iri {
    iri!("http://zakhor/ns/dependsOn")
}

pub fn supersedes_iri() -> &'static Iri {
    iri!("http://zakhor/ns/supersedes")
}

pub fn evidence_for_iri() -> &'static Iri {
    iri!("http://zakhor/ns/evidenceFor")
}

pub fn belongs_to_project_iri() -> &'static Iri {
    iri!("http://zakhor/ns/belongsToProject")
}

pub fn code_location_iri() -> &'static Iri {
    iri!("http://zakhor/ns/codeLocation")
}

// --- Observation properties ---

pub fn observation_content_iri() -> &'static Iri {
    iri!("http://zakhor/ns/observationContent")
}

pub fn observation_created_at_iri() -> &'static Iri {
    iri!("http://zakhor/ns/observationCreatedAt")
}

// --- Tool-call properties ---

pub fn tool_name_iri() -> &'static Iri {
    iri!("http://zakhor/ns/toolName")
}

pub fn tool_arguments_iri() -> &'static Iri {
    iri!("http://zakhor/ns/toolArguments")
}

pub fn session_id_iri() -> &'static Iri {
    iri!("http://zakhor/ns/sessionId")
}

pub fn timestamp_iri() -> &'static Iri {
    iri!("http://zakhor/ns/timestamp")
}

// --- Ranking ---

pub fn graph_importance_iri() -> &'static Iri {
    iri!("http://zakhor/ns/graphImportance")
}

pub fn provenance_quality_iri() -> &'static Iri {
    iri!("http://zakhor/ns/provenanceQuality")
}

// --- Named graph prefix ---

pub const NAMED_GRAPH_PREFIX: &str = "http://zakhor/ns/graph/";

// --- Decision status constants ---

pub mod decision_status {
    pub const ACTIVE: &str = "active";
    pub const SUPERSEDED: &str = "superseded";
}

// --- Tests ---

#[cfg(test)]
mod tests {
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
}
