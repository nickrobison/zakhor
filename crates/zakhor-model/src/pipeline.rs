//! 5-stage ingestion pipeline.
//!
//! Stages: validate, resolve, build, persist, track. The struct lives here with
//! its constructors and stage implementations; entry points are in
//! [`crate::ingest`] and [`crate::ingest_async`].

use gio::Cancellable;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracker::SparqlConnection;
use tracker::prelude::SparqlConnectionExtManual;
use zakhor_search::IndexSyncManager;

use crate::entity_resolver::EntityResolver;
use crate::errors::IngestionError;
use crate::provenance::ProvenanceTracker;
use crate::sparql_builder::{build_observation_sparql, collect_provenance_triples};

/// An entity reference associated with an observation.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct EntityRef {
    pub uri: String,
    pub label: String,
}

/// A relation between two entities.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct Relation {
    pub subject_uri: String,
    pub predicate_uri: String,
    pub object_uri: String,
    pub label: String,
}

/// Arguments for storing a complete observation.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct StoreObservationArgs {
    pub text: String,
    pub entities: Vec<EntityRef>,
    pub relations: Vec<Relation>,
}

/// Result of a successfully ingested observation.
#[derive(Clone, Debug)]
pub struct IngestResult {
    pub observation_uri: String,
    pub triple_count: usize,
}

/// Intermediate state after stages 1–3, shared by sync/async entry points.
pub(crate) struct IngestPrepared {
    pub uuid_urn: String,
    pub sparql: String,
    pub provenance_triples: Vec<(String, String, String)>,
    /// Post-resolve text — needed by the async index-sync stage.
    pub text: String,
    /// Post-resolve entity URIs — needed by the async index-sync stage.
    pub entity_uris: Vec<String>,
}
// ---------------------------------------------------------------------------
// 5-Stage IngestionPipeline
// ---------------------------------------------------------------------------

/// 5-stage ingestion pipeline for persisting observations.
///
/// Stages:
/// 1. **Validate** — Check that input args are well-formed.
/// 2. **Resolve** — Resolve entity labels to canonical URIs (skip if no resolver).
/// 3. **Build** — Construct SPARQL INSERT DATA + collect provenance triples.
/// 4. **Persist** — Execute SPARQL update against the triplestore.
/// 5. **Track** — Track provenance in-memory; optionally sync to search indexes.
pub struct IngestionPipeline {
    pub(crate) provenance: ProvenanceTracker,
    pub(crate) entity_resolver: Option<Arc<EntityResolver>>,
    pub(crate) sync_manager: Option<Arc<IndexSyncManager>>,
}

impl IngestionPipeline {
    pub fn new() -> Self {
        Self {
            provenance: ProvenanceTracker::new(),
            entity_resolver: None,
            sync_manager: None,
        }
    }

    /// Create a pipeline with an optional entity resolver.
    pub fn with_resolver(resolver: Option<Arc<EntityResolver>>) -> Self {
        Self {
            provenance: ProvenanceTracker::new(),
            entity_resolver: resolver,
            sync_manager: None,
        }
    }

    /// Create a pipeline with an optional index sync manager.
    pub fn with_sync_manager(sync_manager: Option<Arc<IndexSyncManager>>) -> Self {
        Self {
            provenance: ProvenanceTracker::new(),
            entity_resolver: None,
            sync_manager,
        }
    }
    // -----------------------------------------------------------------------
    // Stage implementations
    // -----------------------------------------------------------------------

    /// Get the provenance tracker (for querying graph history).
    pub fn provenance(&self) -> &ProvenanceTracker {
        &self.provenance
    }

    /// Stage 1: Validate input args.
    #[tracing::instrument(skip_all)]
    pub(crate) fn validate(&self, args: &StoreObservationArgs) -> Result<(), IngestionError> {
        if args.text.trim().is_empty() {
            return Err(IngestionError::Validation(
                "observation text must not be empty".to_string(),
                "validate",
                None,
            ));
        }
        for entity in &args.entities {
            if entity.uri.trim().is_empty() {
                return Err(IngestionError::Validation(
                    "entity URI must not be empty".to_string(),
                    "validate",
                    None,
                ));
            }
        }
        Ok(())
    }

    /// Stage 2: Resolve entity labels using the entity resolver.
    #[tracing::instrument(skip_all)]
    pub(crate) fn resolve_entities(
        &self,
        args: &mut StoreObservationArgs,
    ) -> Result<(), IngestionError> {
        let resolver = self.entity_resolver.as_ref().ok_or_else(|| {
            IngestionError::Resolution(
                "entity resolver not configured".to_string(),
                "resolve",
                None,
            )
        })?;

        for entity in &mut args.entities {
            if !entity.label.starts_with("http://") && !entity.label.starts_with("urn:") {
                let result = resolver.resolve(&entity.label);
                if let Some(ref uri) = result.resolved_uri {
                    entity.uri = uri.as_str().to_string();
                }
            }
        }
        Ok(())
    }

    /// Stage 3: Build SPARQL query and collect provenance triples.
    #[tracing::instrument(skip_all)]
    pub fn build_triples(
        &self,
        args: &StoreObservationArgs,
        uuid_urn: &str,
    ) -> (String, Vec<(String, String, String)>) {
        let sparql = build_observation_sparql(args, uuid_urn);
        let triples = collect_provenance_triples(args, uuid_urn);
        (sparql, triples)
    }

    /// Stage 4: Persist to SPARQL triplestore.
    #[tracing::instrument(skip_all)]
    pub(crate) fn persist(
        &self,
        conn: &SparqlConnection,
        sparql: &str,
    ) -> Result<(), IngestionError> {
        conn.update(sparql, None::<&Cancellable>).map_err(|e| {
            IngestionError::Persist(
                format!("SPARQL update failed: {}", e),
                "persist",
                Some(Box::new(e)),
            )
        })
    }

    /// Stages 1–3: validate, resolve, build.
    #[tracing::instrument(skip_all, fields(correlation_id = %correlation_id))]
    pub(crate) fn prepare_ingest(
        &mut self,
        mut args: StoreObservationArgs,
        correlation_id: &str,
    ) -> Result<IngestPrepared, IngestionError> {
        self.validate(&args)?;
        tracing::debug!("Stage 1 [validate] passed");

        if self.entity_resolver.is_some() {
            let before = args.entities.len();
            self.resolve_entities(&mut args)?;
            tracing::trace!(
                before = before,
                after = args.entities.len(),
                "Stage 2 [resolve] complete"
            );
        } else {
            tracing::trace!("Stage 2 [resolve] skipped — no resolver configured");
        }

        let uuid_urn: String = tracker::functions::sparql_get_uuid_urn()
            .ok_or_else(|| {
                IngestionError::Build("Failed to generate UUID".to_string(), "build", None)
            })?
            .to_string();
        let (sparql, provenance_triples) = self.build_triples(&args, &uuid_urn);
        tracing::debug!(
            observation_uri = %uuid_urn,
            triple_count = provenance_triples.len(),
            sparql_len = sparql.len(),
            "Stage 3 [build] complete"
        );

        // Capture post-resolve data needed by the async index-sync stage.
        let text = args.text.clone();
        let entity_uris: Vec<String> = args.entities.iter().map(|e| e.uri.clone()).collect();

        Ok(IngestPrepared {
            uuid_urn,
            sparql,
            provenance_triples,
            text,
            entity_uris,
        })
    }

    /// Stage 5: Track provenance in-memory. Returns the triple count.
    pub(crate) fn track_provenance(
        &mut self,
        uuid_urn: &str,
        provenance_triples: Vec<(String, String, String)>,
    ) -> usize {
        let triple_count = provenance_triples.len();
        let uuid_part = uuid_urn.strip_prefix("urn:uuid:").unwrap_or(uuid_urn);
        self.provenance
            .add_observation(uuid_part, provenance_triples);
        tracing::debug!(
            observation_uri = %uuid_urn,
            triple_count,
            "Stage 5 [track] complete"
        );
        triple_count
    }
}

impl Default for IngestionPipeline {
    fn default() -> Self {
        Self::new()
    }
}
// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
