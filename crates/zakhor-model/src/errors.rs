//! Ingestion pipeline error types.

/// Error type for ingestion pipeline stages.
///
/// Each variant carries:
/// - A human-readable message (for Display / user-facing output).
/// - A `stage_name` identifying which pipeline stage produced the error.
/// - An optional `#[source]` wrapping the underlying error when applicable.
#[derive(Debug, thiserror::Error)]
pub enum IngestionError {
    #[error("validation: {0}")]
    Validation(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    #[error("resolution: {0}")]
    Resolution(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    #[error("build: {0}")]
    Build(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    #[error("persist: {0}")]
    Persist(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    #[error("sync: {0}")]
    Sync(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    /// Error from tokio task join (e.g. `spawn_blocking` panicked or was cancelled).
    #[error("join: {0}")]
    Join(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

impl From<IngestionError> for String {
    fn from(e: IngestionError) -> String {
        e.to_string()
    }
}