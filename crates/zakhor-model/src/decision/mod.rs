#![allow(dead_code)]

mod model;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use model::build_create_decision_sparql;
pub use model::*;
