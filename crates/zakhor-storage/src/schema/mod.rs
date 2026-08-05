#![allow(dead_code)]

mod prefixes;
mod ontology;
mod migrations;

#[cfg(test)]
mod tests;

pub use crate::sparql::SparqlBuilder;
pub use prefixes::*;
pub use ontology::*;
pub use migrations::*;
