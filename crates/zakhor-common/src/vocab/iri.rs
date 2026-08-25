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

pub fn repository_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Repository")
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

pub fn belongs_to_repository_iri() -> &'static Iri {
    iri!("http://zakhor/ns/belongsToRepository")
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
