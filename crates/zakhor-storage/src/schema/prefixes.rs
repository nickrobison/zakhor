/// Additional namespace constants (beyond those in Prefix)
pub const NFO: &str = "http://www.semanticdesktop.org/ontologies/2007/03/22/nfo#";
pub const NAO: &str = "http://www.semanticdesktop.org/ontologies/2007/08/15/nao#";
pub const SKOS: &str = "http://www.w3.org/2004/02/skos/core#";
pub const NRL: &str = "http://tracker.api.gnome.org/ontology/v3/nrl#";

/// All prefix entries for SPARQL queries (adds NFO, NAO, SKOS to basic set from sparql.rs)
pub const EXTRA_PREFIXES: &[(&str, &str)] = &[("nfo", NFO), ("nao", NAO), ("skos", SKOS)];

// IRI constructor functions — re-exported from zakhor_common::vocab (single canonical source):

pub use zakhor_common::vocab::{
    belongs_to_project_iri, belongs_to_repository_iri, code_location_iri, conflicts_with_iri,
    constraint_iri, decision_alternative_iri, decision_context_iri, decision_iri,
    decision_outcome_iri, decision_rationale_iri, decision_status_iri, depends_on_iri, entity_iri,
    evidence_for_iri, graph_importance_iri, has_entity_iri, has_relation_iri, issue_iri,
    observation_content_iri, observation_created_at_iri, observation_iri, project_iri,
    provenance_graph_iri, provenance_quality_iri, repository_iri, session_id_iri, supersedes_iri,
    timestamp_iri, tool_arguments_iri, tool_call_iri as schema_tool_call_iri, tool_name_iri,
};
