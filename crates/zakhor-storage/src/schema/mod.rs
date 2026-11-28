#![allow(dead_code)]

mod migrations;
mod prefixes;

#[cfg(test)]
mod tests;

pub use crate::sparql::SparqlBuilder;
pub use migrations::*;
pub use prefixes::*;
