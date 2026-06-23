#![allow(dead_code)]

use crate::sparql::Prefix;
use iref::Iri;
use static_iref::iri;

// --- Decision relations ---

pub fn conflicts_with_iri() -> &'static Iri {
    iri!("http://zakhor/ns/conflictsWith")
}

pub fn depends_on_iri() -> &'static Iri {
    iri!("http://zakhor/ns/dependsOn")
}

pub fn supersedes_iri() -> &'static Iri {
    iri!("http://zakhor/ns/supersedes")
}

// --- Evidence / provenance ---

pub fn evidence_for_iri() -> &'static Iri {
    iri!("http://zakhor/ns/evidenceFor")
}

// --- Project association ---

pub fn belongs_to_project_iri() -> &'static Iri {
    iri!("http://zakhor/ns/belongsToProject")
}

// --- Code indexing ---

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

// --- Decision properties ---

pub fn decision_status_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionStatus")
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
    fn test_decision_status_iri() {
        let iri = decision_status_iri();
        assert!(iri.as_str().ends_with("decisionStatus"));
    }

    #[test]
    fn test_all_predicates_use_zakhor_ns() {
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
        ];
        for iri in &iris {
            assert!(iri.as_str().starts_with(Prefix::ZAKHOR));
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
