//! Ingestion pipeline async entry points.
//!
//! The 5-stage pipeline body lives in [`crate::pipeline::IngestionPipeline`];
//! this module holds the asynchronous entry-point methods.

use gio::Cancellable;
use std::sync::Arc;
use tracker::SparqlConnection;
use tracker::prelude::SparqlConnectionExtManual;

use crate::errors::IngestionError;
use crate::extraction::ExtractionPipeline;
use crate::pipeline::{IngestResult, IngestionPipeline, StoreObservationArgs};

impl IngestionPipeline {
    pub async fn ingest_async(
        &mut self,
        conn: Arc<SparqlConnection>,
        args: StoreObservationArgs,
        correlation_id: &str,
    ) -> Result<IngestResult, IngestionError> {
        let prepared = self.prepare_ingest(args, correlation_id)?;

        // Stage 4: Persist + Index Sync (concurrent via tokio::join!)
        let persist_conn = conn.clone();
        let sync_manager = self.sync_manager.clone();
        let uuid_urn_for_sync = prepared.uuid_urn.clone();
        let text = prepared.text.clone();
        let entity_uris = prepared.entity_uris.clone();

        let persist_fut = tokio::task::spawn_blocking(move || {
            persist_conn
                .update(&prepared.sparql, None::<&Cancellable>)
                .map_err(|e| {
                    IngestionError::Persist(
                        format!("SPARQL update failed: {}", e),
                        "persist",
                        Some(Box::new(e)),
                    )
                })
        });

        let sync_fut = tokio::task::spawn_blocking(move || {
            if let Some(ref mgr) = sync_manager {
                mgr.sync_observation(&uuid_urn_for_sync, &text, &entity_uris)
                    .map_err(|e| {
                        IngestionError::Sync(format!("index sync failed: {}", e), "sync", None)
                    })
            } else {
                Ok(())
            }
        });

        let (persist_result, sync_result) = tokio::join!(persist_fut, sync_fut);

        // Handle persist result (must succeed — propagate errors)
        match persist_result {
            Err(join_err) => {
                return Err(IngestionError::Join(
                    format!("persist task panicked: {}", join_err),
                    "join",
                    Some(Box::new(join_err)),
                ));
            }
            Ok(Err(ingest_err)) => {
                tracing::error!(error = %ingest_err, "Stage 4 [persist] failed");
                return Err(ingest_err);
            }
            Ok(Ok(())) => {
                tracing::debug!("Stage 4 [persist] SPARQL update succeeded");
            }
        }

        // Handle sync result (best-effort — log warning, don't fail)
        match sync_result {
            Err(join_err) => {
                tracing::warn!(error = %join_err, "Index sync task panicked (non-fatal)");
            }
            Ok(Err(sync_err)) => {
                tracing::warn!(error = %sync_err, "Index sync failed (non-fatal)");
            }
            Ok(Ok(())) => {
                tracing::debug!("Stage 4 [sync] index sync succeeded");
            }
        }

        // Stage 5: Track
        let triple_count = self.track_provenance(&prepared.uuid_urn, prepared.provenance_triples);

        Ok(IngestResult {
            observation_uri: prepared.uuid_urn,
            triple_count,
        })
    }

    pub async fn extract_and_ingest_async(
        &mut self,
        conn: Arc<SparqlConnection>,
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
            "Starting 5-stage async ingest from extracted results"
        );
        self.ingest_async(conn, args, correlation_id).await
    }
}
