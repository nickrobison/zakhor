//! ONNX-based GLiNER extraction pipeline for entity and relation extraction.
//!
//! Wraps [`gliner`](https://github.com/fbilhaut/gline-rs) (gline-rs) with a
//! [`tokio::task::spawn_blocking`] boundary so that CPU-bound ONNX inference
//! does not block the async runtime.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────┐
//! │  ExtractionPipeline                            │
//! │  ┌──────────────┐   ┌──────────────────────┐   │
//! │  │ extract_     │   │ extract_             │   │
//! │  │ entities()   │   │ relations()          │   │
//! │  └──────┬───────┘   └──────────┬───────────┘   │
//! │         │                      │               │
//! │         ▼                      ▼               │
//! │  ┌──────────────┐   ┌──────────────────────┐   │
//! │  │ GLiNER       │   │ Model::inference     │   │
//! │  │ <TokenMode>  │   │ (NER → RE chain)     │   │
//! │  └──────────────┘   └──────────────────────┘   │
//! │         │                      │               │
//! │         ▼                      ▼               │
//! │  ┌──────────────┐   ┌──────────────────────┐   │
//! │  │ Vec<EntityRef>│   │ Vec<Relation>        │   │
//! │  └──────────────┘   └──────────────────────┘   │
//! └────────────────────────────────────────────────┘
//! ```
//!
//! All ONNX model interaction happens inside `tokio::task::spawn_blocking`
//! so the async executor is never blocked by inference.

mod config;
mod errors;
mod pipeline;

#[cfg(test)]
mod pipeline_tests;

pub use config::ExtractionConfig;
pub use errors::ExtractionError;
pub use pipeline::ExtractionPipeline;
