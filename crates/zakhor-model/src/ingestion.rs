//! Ingestion pipeline (re-export shim).
//!
//! The pipeline was split into focused modules; this file preserves the
//! historical `zakhor_model::ingestion` path for compatibility.

pub use crate::errors::IngestionError;
pub use crate::ingest::*;
pub use crate::ingest_async::*;
pub use crate::pipeline::{
    EntityRef, IngestResult, IngestionPipeline, Relation, StoreObservationArgs,
};
pub use crate::sparql_builder::{
    build_observation_sparql, collect_provenance_triples, escape_literal, format_iri,
};
