#![allow(dead_code)]

mod builder;
mod escape;
mod prefix;
mod queries;

#[cfg(test)]
mod tests;

pub use builder::SparqlBuilder;
pub use escape::escape_literal;
pub use escape::format_iri;
pub use prefix::prefix_declarations;
pub use prefix::Prefix;
#[cfg(test)]
pub(crate) use prefix::PREFIX_LIST;
pub use queries::ontology_construct;
