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
use zakhor_search::IndexSyncManager;

impl IngestionPipeline {
    pub async fn ingest_async(
        &mut self,
        conn: Arc<SparqlConnection>,
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
            tracing::trace!("Stage 2 [resolve] skipped \u{2014} no resolver configured");
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

        // Extract data for index sync (after resolve stage may have mutated entities)
        let text = args.text.clone();
        let entity_uris: Vec<String> = args.entities.iter().map(|e| e.uri.clone()).collect();

        // Stage 4: Persist + Index Sync (concurrent via tokio::join!)
        let persist_conn = conn.clone();
        let sync_manager = self.sync_manager.clone();
        let uuid_urn_for_sync = uuid_urn.clone();

        let persist_fut = tokio::task::spawn_blocking(move || {
            persist_conn
                .update(&sparql, None::<&Cancellable>)
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