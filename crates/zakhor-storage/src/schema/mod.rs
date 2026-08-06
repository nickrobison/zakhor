#![allow(dead_code)]

mod migrations;
mod ontology;
mod prefixes;

#[cfg(test)]
mod tests;

pub use crate::sparql::SparqlBuilder;
pub use migrations::*;
pub use ontology::*;
pub use prefixes::*;
