#![allow(dead_code)]

mod builder;
mod escape;
mod prefix;

#[cfg(test)]
mod tests;

pub use builder::SparqlBuilder;
pub use escape::InvalidIri;
pub use escape::escape_literal;
pub use escape::format_iri;
pub use escape::validate_iri;
pub use prefix::Prefix;
pub use prefix::prefix_declarations;
