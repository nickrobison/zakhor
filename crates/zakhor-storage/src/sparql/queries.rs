use super::builder::SparqlBuilder;

/// Build a CONSTRUCT query for ontology registration.
///
/// `construct_pattern` and `where_pattern` are arbitrary triple-pattern
/// fragments (prefixed names are resolved by the `PREFIX` declarations).
pub fn ontology_construct(construct_pattern: &str, where_pattern: &str) -> String {
    SparqlBuilder::construct(construct_pattern, where_pattern)
}
