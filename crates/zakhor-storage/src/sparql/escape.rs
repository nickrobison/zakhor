use oxrdf::{Literal, NamedNode};

/// A URI that could not be parsed as an IRI.
///
/// This exists because the alternative — stripping angle brackets and hoping —
/// accepts input that is not addressable and then fails deep inside the storage
/// layer, where the resulting parse error mentions byte offsets rather than the
/// argument the caller actually got wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidIri(pub String);

impl std::fmt::Display for InvalidIri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} is not a valid URI", self.0)
    }
}

impl std::error::Error for InvalidIri {}

/// Escape `text` as a SPARQL literal using `oxrdf::Literal`.
/// The returned string includes the enclosing double quotes and any internal
/// escaping — it is safe to interpolate directly into a SPARQL query string.
pub fn escape_literal(text: &str) -> String {
    let lit = Literal::new_simple_literal(text);
    lit.to_string()
}

/// Validate `iri` and return it wrapped in angle brackets.
///
/// Rejecting unparseable input is what actually protects the query structure.
/// Stripping angle brackets is not sufficient on its own: it keeps the framing
/// intact but still lets a caller supply an IRI containing whitespace, quotes or
/// a NUL byte, none of which belong in an address.
pub fn format_iri(iri_str: &str) -> Result<String, InvalidIri> {
    NamedNode::new(iri_str)
        .map(|node| node.to_string())
        .map_err(|_| InvalidIri(iri_str.to_string()))
}

/// Check that `iri_str` is addressable without producing a formatted IRI.
pub fn validate_iri(iri_str: &str) -> Result<(), InvalidIri> {
    NamedNode::new(iri_str)
        .map(|_| ())
        .map_err(|_| InvalidIri(iri_str.to_string()))
}
