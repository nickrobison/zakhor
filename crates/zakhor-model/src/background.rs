//! Background Worker Framework (Phase 3.1)
//!
//! Spawns Tokio tasks that periodically perform maintenance:
//!   - Recompute graph importance rankings
//!   - Clean up stale Tombstone / superseded decisions
//!   - (future) Evict old named graphs

use gio::Cancellable;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tracker::SparqlConnection;
use tracker::prelude::{SparqlConnectionExtManual, SparqlCursorExtManual};
use zakhor_storage::sparql::Prefix;

/// Configuration for periodic background tasks.
pub struct BackgroundConfig {
    pub rank_interval: Duration,
    pub cleanup_interval: Duration,
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            rank_interval: Duration::from_secs(3600),     // every hour
            cleanup_interval: Duration::from_secs(86400), // once a day
        }
    }
}

/// Start background workers on a cloned SPARQL connection.
///
/// Returns a `Notify` handle that sends a signal when the background
/// loop exits (e.g. during shutdown).
pub fn start_background_workers(conn: SparqlConnection, config: BackgroundConfig) -> Arc<Notify> {
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    tokio::task::spawn(async move {
        let mut rank_timer = tokio::time::interval(config.rank_interval);
        let mut cleanup_timer = tokio::time::interval(config.cleanup_interval);

        // Skip the immediate first tick
        rank_timer.tick().await;
        cleanup_timer.tick().await;

        loop {
            tokio::select! {
                _ = rank_timer.tick() => {
                    if let Err(e) = refresh_ranking(&conn) {
                        tracing::warn!("Background ranking refresh failed: {e}");
                    }
                }
                _ = cleanup_timer.tick() => {
                    match cleanup_stale_data(&conn) {
                        Ok(outcome) if outcome.tombstoned > 0 => {
                            tracing::info!(
                                "Tombstoned {} superseded decisions; {} stale",
                                outcome.tombstoned,
                                outcome.stale
                            );
                        }
                        Ok(_) => {}
                        Err(e) => tracing::warn!("Background cleanup failed: {e}"),
                    }
                }
                _ = shutdown_clone.notified() => {
                    tracing::info!("Background workers shutting down");
                    break;
                }
            }
        }
    });

    shutdown
}

/// Refresh graph importance and provenance quality scores for all entities.
fn refresh_ranking(conn: &SparqlConnection) -> Result<(), String> {
    let entities = crate::ranking::compute_importance(conn)?;
    for entity in &entities {
        let safe_uri = entity.uri.as_str().replace('>', "");
        let sparql = format!(
            r#"PREFIX zakhor: <{ns}>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>

DELETE {{ <{uri}> zakhor:graphImportance ?old . }}
INSERT {{ <{uri}> zakhor:graphImportance "{score}"^^xsd:float . }}
WHERE {{ OPTIONAL {{ <{uri}> zakhor:graphImportance ?old . }} }}"#,
            ns = Prefix::ZAKHOR,
            uri = safe_uri,
            score = entity.importance,
        );
        if let Err(e) = conn.update(&sparql, None::<&Cancellable>) {
            tracing::warn!("Failed to update ranking for {}: {e}", entity.uri);
        }
    }
    Ok(())
}

/// Outcome of a cleanup pass, for logging and tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CleanupOutcome {
    /// Superseded decisions that were missing a tombstone and got one this
    /// pass (at most [`CLEANUP_BATCH_LIMIT`]).
    pub tombstoned: u64,
    /// Superseded decisions whose tombstone is older than the staleness
    /// window — candidates for future eviction.
    pub stale: u64,
}

/// How many missing tombstones one cleanup pass may write.
const CLEANUP_BATCH_LIMIT: u32 = 100;

/// A decision counts as stale once it has been superseded for this long.
const STALENESS_WINDOW_DAYS: i64 = 30;

/// Maintenance pass over superseded decisions.
///
/// `zakhor:tombstone` records *when* a decision was superseded (written by
/// [`zakhor_model::decision::DecisionModel::supersede`]). This pass:
///
/// 1. Backfills the tombstone for superseded decisions that lack one (e.g.
///    rows written before this predicate existed), using the current time —
///    the original transition time is unknowable, so the staleness window
///    starts now.
/// 2. Reports decisions superseded for more than [`STALENESS_WINDOW_DAYS`]
///    days as stale, so operators (and the future eviction job) can see what
///    is eligible for removal. Nothing is deleted: cleanup never mutates or
///    drops user data beyond stamping tombstones.
pub fn cleanup_stale_data(conn: &SparqlConnection) -> Result<CleanupOutcome, String> {
    let mut outcome = CleanupOutcome::default();

    // 1. Backfill tombstones (bounded per pass; the rest waits for the next).
    let missing = format!(
        r#"PREFIX zakhor: <{ns}>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>

SELECT ?decision WHERE {{
  ?decision rdf:type zakhor:Decision .
  ?decision zakhor:decisionStatus "superseded" .
  FILTER NOT EXISTS {{ ?decision zakhor:tombstone ?any_tombstone . }}
}}
LIMIT {limit}"#,
        ns = Prefix::ZAKHOR,
        limit = CLEANUP_BATCH_LIMIT,
    );
    let cursor = conn
        .query(&missing, None::<&Cancellable>)
        .map_err(|e| format!("Stale data query failed: {e}"))?;
    while cursor
        .next(None::<&Cancellable>)
        .map_err(|e| format!("Cursor error: {e}"))?
    {
        let uri = cursor
            .string(0)
            .map(|s| s.to_string())
            .ok_or("stale-data query returned a NULL decision URI")?;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let update = format!(
            "PREFIX zakhor: <{ns}> PREFIX xsd: <http://www.w3.org/2001/XMLSchema#> \
             INSERT DATA {{ <{uri}> zakhor:tombstone \"{now}\"^^xsd:dateTime . }}",
            ns = Prefix::ZAKHOR,
            uri = uri,
            now = now,
        );
        conn.update(&update, None::<&Cancellable>)
            .map_err(|e| format!("Failed to tombstone {uri}: {e}"))?;
        outcome.tombstoned += 1;
    }

    // 2. Report decisions superseded longer than the staleness window.
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(STALENESS_WINDOW_DAYS))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let stale = format!(
        r#"PREFIX zakhor: <{ns}>
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>

SELECT ?decision WHERE {{
  ?decision zakhor:decisionStatus "superseded" .
  ?decision zakhor:tombstone ?tombstone .
  FILTER(?tombstone < "{cutoff}"^^xsd:dateTime)
}}"#,
        ns = Prefix::ZAKHOR,
        cutoff = cutoff,
    );
    let cursor = conn
        .query(&stale, None::<&Cancellable>)
        .map_err(|e| format!("Stale decision query failed: {e}"))?;
    while cursor
        .next(None::<&Cancellable>)
        .map_err(|e| format!("Cursor error: {e}"))?
    {
        if let Some(uri) = cursor.string(0).map(|s| s.to_string()) {
            tracing::warn!(decision = %uri, "superseded decision is stale (>{STALENESS_WINDOW_DAYS} days); eviction is not implemented yet");
            outcome.stale += 1;
        }
    }

    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_background_config_default() {
        let cfg = BackgroundConfig::default();
        assert_eq!(cfg.rank_interval, Duration::from_secs(3600));
        assert_eq!(cfg.cleanup_interval, Duration::from_secs(86400));
    }
}
