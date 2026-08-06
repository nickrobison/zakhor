//! Ingestion pipeline sync entry points.
//!
//! The 5-stage pipeline body lives in [`crate::pipeline::IngestionPipeline`];
//! this module holds the synchronous entry-point methods.

use tracker::SparqlConnection;

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
        let prepared = self.prepare_ingest(args, correlation_id)?;

        // Stage 4: Persist
        self.persist(conn, &prepared.sparql)?;
        tracing::debug!("Stage 4 [persist] SPARQL update succeeded");

        // Stage 5: Track
        let triple_count = self.track_provenance(&prepared.uuid_urn, prepared.provenance_triples);

        Ok(IngestResult {
            observation_uri: prepared.uuid_urn,
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
