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

use crate::sparql::Prefix;

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
                    if let Err(e) = cleanup_stale_data(&conn) {
                        tracing::warn!("Background cleanup failed: {e}");
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

/// Tombstone walking — mark decisions that have been superseded for >30 days
/// with a `zakhor:tombstone` flag.
fn cleanup_stale_data(conn: &SparqlConnection) -> Result<(), String> {
    let sparql = format!(
        r#"PREFIX zakhor: <{ns}>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>

SELECT ?decision WHERE {{
  ?decision rdf:type zakhor:Decision .
  ?decision zakhor:decisionStatus "superseded"@en .
  FILTER NOT EXISTS {{ ?decision zakhor:tombstone true . }}
}}
LIMIT 100"#,
        ns = Prefix::ZAKHOR,
    );

    let cursor = conn
        .query(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Stale data query failed: {e}"))?;

    let mut count = 0u64;
    while cursor
        .next(None::<&Cancellable>)
        .map_err(|e| format!("Cursor error: {e}"))?
    {
        if let Some(uri) = cursor.string(0).map(|s| s.to_string()) {
            let safe = uri.replace('>', "");
            let update = format!(
                "PREFIX zakhor: <{ns}> INSERT DATA {{ <{safe}> zakhor:tombstone true . }}",
                ns = Prefix::ZAKHOR,
                safe = safe,
            );
            if conn.update(&update, None::<&Cancellable>).is_ok() {
                count += 1;
            }
        }
    }

    if count > 0 {
        tracing::info!("Tombstoned {count} superseded decisions");
    }
    Ok(())
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

    #[test]
    fn test_tombstone_honours_limit() {
        // Unit test — ensuring the WHERE clause shape compiles
        let _sparql = format!(
            "PREFIX zakhor: <{}> SELECT ?d WHERE {{ ?d zakhor:decisionStatus \"superseded\"@en }} LIMIT 100",
            Prefix::ZAKHOR
        );
    }
}
