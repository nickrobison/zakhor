#![allow(dead_code)]

use gio::Cancellable;
use tracker::SparqlConnection;
use tracker::prelude::{SparqlConnectionExtManual, SparqlCursorExtManual};

use crate::sparql::Prefix;
use crate::vocab;

/// Arguments for creating a new Decision directly (no Candidate/Proposed).
#[derive(Clone, Debug)]
pub struct CreateDecisionArgs {
    /// Free-form context description.
    pub context: String,
    /// The decision outcome.
    pub outcome: String,
    /// Considered alternatives.
    pub alternatives: Vec<String>,
    /// Rationale for the decision.
    pub rationale: String,
    /// URIs of entities/observations affected by this decision.
    pub affects: Vec<String>,
    /// URIs of observations this decision derives from.
    pub derived_from: Vec<String>,
    /// Optional URI of a superseded decision.
    pub supersedes: Option<String>,
    /// Optional URIs of conflicting decisions.
    pub conflicts_with: Vec<String>,
    /// Optional URIs of decisions this depends on.
    pub depends_on: Vec<String>,
    /// Optional project URI this decision belongs to.
    pub project_uri: Option<String>,
}

/// Result of creating a Decision.
#[derive(Clone, Debug)]
pub struct CreateDecisionResult {
    pub decision_uri: String,
    pub status: String,
}

/// The direct Decision model.
///
/// Decisions are created directly with `active` status (no Candidate/Proposed
/// states). They can be related via `supersedes`, `conflictsWith`, and
/// `dependsOn` edges. Status transitions: active -> superseded (when a newer
/// Decision supersedes this one).
pub struct DecisionModel;

impl DecisionModel {
    /// Create a new Decision directly with `active` status.
    ///
    /// Returns the decision URI and status.
    pub fn create(
        conn: &SparqlConnection,
        args: CreateDecisionArgs,
    ) -> Result<CreateDecisionResult, String> {
        let uuid = tracker::functions::sparql_get_uuid_urn()
            .ok_or_else(|| "Failed to generate UUID".to_string())?
            .to_string();

        let sparql = build_create_decision_sparql(&args, &uuid, &uuid);
        conn.update(&sparql, None::<&Cancellable>)
            .map_err(|e| format!("Failed to create decision: {}", e))?;

        Ok(CreateDecisionResult {
            decision_uri: uuid,
            status: vocab::decision_status::ACTIVE.to_string(),
        })
    }

    /// Supersede an existing decision (set its status to superseded).
    pub fn supersede(conn: &SparqlConnection, decision_uri: &str) -> Result<(), String> {
        let superseded_lit = crate::sparql::escape_literal(vocab::decision_status::SUPERSEDED);
        let sparql = format!(
            "{}DELETE {{ <{}> <{}> ?old_status . }} INSERT {{ <{}> <{}> {} . }} WHERE {{ <{}> <{}> ?old_status . }}",
            crate::sparql::prefix_declarations(),
            decision_uri,
            vocab::decision_status_iri().as_str(),
            decision_uri,
            vocab::decision_status_iri().as_str(),
            superseded_lit,
            decision_uri,
            vocab::decision_status_iri().as_str(),
        );
        conn.update(&sparql, None::<&Cancellable>)
            .map_err(|e| format!("Failed to supersede decision: {}", e))
    }

    /// Query decisions by status.
    pub fn query_by_status(
        conn: &SparqlConnection,
        status: &str,
        limit: u32,
    ) -> Result<Vec<String>, String> {
        let status_lit = crate::sparql::escape_literal(status);
        let sparql = format!(
            "{}SELECT ?d WHERE {{ ?d rdf:type <{}> ; <{}> {} . }} LIMIT {}",
            crate::sparql::prefix_declarations(),
            crate::schema::decision_iri().as_str(),
            vocab::decision_status_iri().as_str(),
            status_lit,
            limit,
        );
        let cursor = conn
            .query(&sparql, None::<&Cancellable>)
            .map_err(|e| format!("SPARQL query failed: {}", e))?;

        let mut results = Vec::new();
        while cursor
            .next(None::<&Cancellable>)
            .map_err(|e| format!("Cursor error: {}", e))?
        {
            if let Some(s) = cursor.string(0) {
                results.push(s.to_string());
            }
        }
        Ok(results)
    }
}

/// Build SPARQL INSERT for creating a new Decision.
fn build_create_decision_sparql(
    args: &CreateDecisionArgs,
    decision_uri: &str,
    _uuid: &str,
) -> String {
    let mut sparql = String::with_capacity(2048);
    sparql.push_str(&crate::sparql::prefix_declarations());
    sparql.push_str("INSERT DATA {\n");

    // Decision node with type and status
    let context_lit = crate::sparql::escape_literal(&args.context);
    let outcome_lit = crate::sparql::escape_literal(&args.outcome);
    let rationale_lit = crate::sparql::escape_literal(&args.rationale);
    let status_lit = crate::sparql::escape_literal(vocab::decision_status::ACTIVE);

    sparql.push_str(&format!(
        "  <{}> rdf:type <{}> ;\n              <{}> {} ;\n              <{}> {} ;\n              <{}> {} ;\n              <{}> {} .\n",
        decision_uri,
        crate::schema::decision_iri().as_str(),
        crate::schema::decision_context_iri().as_str(), context_lit,
        crate::schema::decision_outcome_iri().as_str(), outcome_lit,
        crate::schema::decision_rationale_iri().as_str(), rationale_lit,
        vocab::decision_status_iri().as_str(), status_lit,
    ));

    // Alternatives
    for alt in &args.alternatives {
        let alt_lit = crate::sparql::escape_literal(alt);
        sparql.push_str(&format!(
            "  <{}> <{}> {} .\n",
            decision_uri,
            crate::schema::decision_alternative_iri().as_str(),
            alt_lit,
        ));
    }

    // Affects edges
    for aff in &args.affects {
        sparql.push_str(&format!(
            "  <{}> <{}> <{}> .\n",
            decision_uri,
            crate::schema::provenance_graph_iri().as_str(),
            aff,
        ));
    }

    // prov:wasDerivedFrom
    for df in &args.derived_from {
        sparql.push_str(&format!(
            "  <{}> <{}> <{}> .\n",
            decision_uri,
            Prefix::PROV_WAS_DERIVED_FROM,
            df,
        ));
    }

    // Supersedes
    if let Some(ref s) = args.supersedes {
        sparql.push_str(&format!(
            "  <{}> <{}> <{}> .\n",
            decision_uri,
            vocab::supersedes_iri().as_str(),
            s,
        ));
    }

    // Conflicts with
    for cw in &args.conflicts_with {
        sparql.push_str(&format!(
            "  <{}> <{}> <{}> .\n",
            decision_uri,
            vocab::conflicts_with_iri().as_str(),
            cw,
        ));
    }

    // Depends on
    for dpo in &args.depends_on {
        sparql.push_str(&format!(
            "  <{}> <{}> <{}> .\n",
            decision_uri,
            vocab::depends_on_iri().as_str(),
            dpo,
        ));
    }

    // Project association
    if let Some(ref project) = args.project_uri {
        sparql.push_str(&format!(
            "  <{}> <{}> <{}> .\n",
            decision_uri,
            vocab::belongs_to_project_iri().as_str(),
            project,
        ));
    }

    sparql.push_str("}\n");
    sparql
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_decision_args_struct() {
        let args = CreateDecisionArgs {
            context: "Test context".into(),
            outcome: "Approved".into(),
            alternatives: vec!["Alt A".into(), "Alt B".into()],
            rationale: "Because".into(),
            affects: vec!["http://zakhor/ns/entity/e1".into()],
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
        let sparql = build_create_decision_sparql(&args, "http://zakhor/ns/decision/test-1", "");
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
            affects: vec!["http://zakhor/ns/entity/e1".into()],
            derived_from: vec!["urn:uuid:obs-1".into()],
            supersedes: Some("http://zakhor/ns/decision/old".into()),
            conflicts_with: vec!["http://zakhor/ns/decision/conflict".into()],
            depends_on: vec!["http://zakhor/ns/decision/dep".into()],
            project_uri: Some("http://zakhor/ns/project/p1".into()),
        };
        let sparql = build_create_decision_sparql(&args, "http://zakhor/ns/decision/test-2", "");
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
}
