#![allow(dead_code)]

use gio::Cancellable;
use oxiri::Iri;
use tracker::SparqlConnection;
use tracker::prelude::{SparqlConnectionExtManual, SparqlCursorExtManual};
use zakhor_common::vocab;
use zakhor_storage::schema;
use zakhor_storage::sparql::{self as storage_sparql, Prefix};

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
    pub affects: Vec<Iri<String>>,
    /// URIs of observations this decision derives from.
    pub derived_from: Vec<Iri<String>>,
    /// Optional URI of a superseded decision.
    pub supersedes: Option<Iri<String>>,
    /// Optional URIs of conflicting decisions.
    pub conflicts_with: Vec<Iri<String>>,
    /// Optional URIs of decisions this depends on.
    pub depends_on: Vec<Iri<String>>,
    /// Optional project URI this decision belongs to.
    pub project_uri: Option<Iri<String>>,
}

/// Result of creating a Decision.
#[derive(Clone, Debug)]
pub struct CreateDecisionResult {
    pub decision_uri: Iri<String>,
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
        let decision_uri_string = tracker::functions::sparql_get_uuid_urn()
            .ok_or_else(|| "Failed to generate UUID".to_string())?
            .to_string();
        let decision_uri = Iri::parse(decision_uri_string)
            .map_err(|e| format!("Generated invalid decision URI: {e}"))?;

        let sparql = build_create_decision_sparql(&args, &decision_uri);
        conn.update(&sparql, None::<&Cancellable>)
            .map_err(|e| format!("Failed to create decision: {}", e))?;

        Ok(CreateDecisionResult {
            decision_uri,
            status: vocab::decision_status::ACTIVE.to_string(),
        })
    }

    /// Supersede an existing decision (set its status to superseded).
    pub fn supersede(conn: &SparqlConnection, decision_uri: &Iri<String>) -> Result<(), String> {
        let superseded_lit = storage_sparql::escape_literal(vocab::decision_status::SUPERSEDED);
        let sparql = format!(
            "{}DELETE {{ <{}> <{}> ?old_status . }} INSERT {{ <{}> <{}> {} . }} WHERE {{ <{}> <{}> ?old_status . }}",
            storage_sparql::prefix_declarations(),
            decision_uri.as_str(),
            vocab::decision_status_iri().as_str(),
            decision_uri.as_str(),
            vocab::decision_status_iri().as_str(),
            superseded_lit,
            decision_uri.as_str(),
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
    ) -> Result<Vec<Iri<String>>, String> {
        let status_lit = storage_sparql::escape_literal(status);
        let sparql = format!(
            "{}SELECT ?d WHERE {{ ?d rdf:type <{}> ; <{}> {} . }} LIMIT {}",
            storage_sparql::prefix_declarations(),
            schema::decision_iri().as_str(),
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
                let iri = Iri::parse(s.to_string())
                    .map_err(|e| format!("Invalid decision URI returned from query: {e}"))?;
                results.push(iri);
            }
        }
        Ok(results)
    }
}

/// Build SPARQL INSERT for creating a new Decision.
pub(crate) fn build_create_decision_sparql(
    args: &CreateDecisionArgs,
    decision_uri: &Iri<String>,
) -> String {
    let mut sparql = String::with_capacity(2048);
    sparql.push_str(&storage_sparql::prefix_declarations());
    sparql.push_str("INSERT DATA {\n");

    // Decision node with type and status
    let context_lit = storage_sparql::escape_literal(&args.context);
    let outcome_lit = storage_sparql::escape_literal(&args.outcome);
    let rationale_lit = storage_sparql::escape_literal(&args.rationale);
    let status_lit = storage_sparql::escape_literal(vocab::decision_status::ACTIVE);

    sparql.push_str(&format!(
        "  <{}> rdf:type <{}> ;\n              <{}> {} ;\n              <{}> {} ;\n              <{}> {} ;\n              <{}> {} .\n",
        decision_uri.as_str(),
        schema::decision_iri().as_str(),
        schema::decision_context_iri().as_str(), context_lit,
        schema::decision_outcome_iri().as_str(), outcome_lit,
        schema::decision_rationale_iri().as_str(), rationale_lit,
        vocab::decision_status_iri().as_str(), status_lit,
    ));

    // Alternatives
    for alt in &args.alternatives {
        let alt_lit = storage_sparql::escape_literal(alt);
        sparql.push_str(&format!(
            "  <{}> <{}> {} .\n",
            decision_uri,
            schema::decision_alternative_iri().as_str(),
            alt_lit,
        ));
    }

    // Affects edges
    for aff in &args.affects {
        sparql.push_str(&format!(
            "  <{}> <{}> <{}> .\n",
            decision_uri.as_str(),
            schema::provenance_graph_iri().as_str(),
            aff.as_str(),
        ));
    }

    // prov:wasDerivedFrom
    for df in &args.derived_from {
        sparql.push_str(&format!(
            "  <{}> <{}> <{}> .\n",
            decision_uri.as_str(),
            Prefix::PROV_WAS_DERIVED_FROM,
            df.as_str(),
        ));
    }

    // Supersedes
    if let Some(ref s) = args.supersedes {
        sparql.push_str(&format!(
            "  <{}> <{}> <{}> .\n",
            decision_uri.as_str(),
            vocab::supersedes_iri().as_str(),
            s.as_str(),
        ));
    }

    // Conflicts with
    for cw in &args.conflicts_with {
        sparql.push_str(&format!(
            "  <{}> <{}> <{}> .\n",
            decision_uri.as_str(),
            vocab::conflicts_with_iri().as_str(),
            cw.as_str(),
        ));
    }

    // Depends on
    for dpo in &args.depends_on {
        sparql.push_str(&format!(
            "  <{}> <{}> <{}> .\n",
            decision_uri.as_str(),
            vocab::depends_on_iri().as_str(),
            dpo.as_str(),
        ));
    }

    // Project association
    if let Some(ref project) = args.project_uri {
        sparql.push_str(&format!(
            "  <{}> <{}> <{}> .\n",
            decision_uri.as_str(),
            vocab::belongs_to_project_iri().as_str(),
            project.as_str(),
        ));
    }

    sparql.push_str("}\n");
    sparql
}
