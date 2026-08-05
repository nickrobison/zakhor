//! Ingestion pipeline sync entry points.
//!
//! The 5-stage pipeline body lives in [`crate::pipeline::IngestionPipeline`];
//! this module holds the synchronous entry-point methods.

use gio::Cancellable;
use tracker::SparqlConnection;
use tracker::prelude::SparqlConnectionExtManual;

use crate::errors::IngestionError;
use crate::extraction::ExtractionPipeline;
use crate::pipeline::{IngestResult, IngestionPipeline, StoreObservationArgs};

impl IngestionPipeline {
    #[tracing::instrument(skip_all, fields(correlation_id = %correlation_id))]
    pub fn ingest(
        &mut self,
        conn: &SparqlConnection,
        args: StoreObservationArgs,
        correlation_id: &str,
    ) -> Result<IngestResult, IngestionError> {
        // Stage 1: Validate
        self.validate(&args)?;
        tracing::debug!("Stage 1 [validate] passed");

        // Stage 2: Resolve (mutate args in place if resolver is available)
        let mut args = args;
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

        // Stage 3: Build
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

        // Stage 4: Persist
        self.persist(conn, &sparql)?;
        tracing::debug!("Stage 4 [persist] SPARQL update succeeded");

        // Stage 5: Track
        let triple_count = provenance_triples.len();
        let uuid_part = uuid_urn.strip_prefix("urn:uuid:").unwrap_or(&uuid_urn);
        self.provenance
            .add_observation(uuid_part, provenance_triples);
        tracing::debug!(
            observation_uri = %uuid_urn,
            triple_count,
            "Stage 5 [track] complete"
        );

        Ok(IngestResult {
            observation_uri: uuid_urn,
            triple_count,
        })
    }

    /// Convenience: ingest + flush + return result.
    /// Flushes the in-memory provenance tracker to the SPARQL store.
    #[tracing::instrument(skip_all, fields(correlation_id = %correlation_id))]
    pub fn ingest_and_flush(
        &mut self,
        conn: &SparqlConnection,
        args: StoreObservationArgs,
        correlation_id: &str,
    ) -> Result<IngestResult, IngestionError> {
        let result = self.ingest(conn, args, correlation_id)?;
        self.provenance.flush_to_sparql(conn).map_err(|e| {
            IngestionError::Persist(format!("flush failed: {}", e), "persist", None)
        })?;
        Ok(result)
    }
    pub async fn extract_and_ingest(
        &mut self,
        conn: &SparqlConnection,
        text: &str,
        extraction: &ExtractionPipeline,
        correlation_id: &str,
    ) -> Result<IngestResult, IngestionError> {
        let text_len = text.len();

        let (entities, relations) = extraction
            .extract_entities_and_relations(text, correlation_id)
            .await
            .map_err(|e| {
                IngestionError::Build(
                    format!("extraction failed: {}", e),
                    "build",
                    Some(Box::new(e)),
                )
            })?;
        tracing::debug!(
            entity_count = entities.len(),
            relation_count = relations.len(),
            "NER+RE extraction complete (shared pass)"
        );

        let args = StoreObservationArgs {
            text: text.to_string(),
            entities,
            relations,
        };

        tracing::info!(
            text_len,
            entity_count = args.entities.len(),
            relation_count = args.relations.len(),
            "Starting 5-stage ingest from extracted results"
        );
        self.ingest(conn, args, correlation_id)
    }
}