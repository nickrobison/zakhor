use oxrdf::{Literal, NamedNode};

/// Escape `text` as a SPARQL literal using `oxrdf::Literal`.
/// The returned string includes the enclosing double quotes and any internal
/// escaping — it is safe to interpolate directly into a SPARQL query string.
pub fn escape_literal(text: &str) -> String {
    let lit = Literal::new_simple_literal(text);
    lit.to_string()
}

/// Format a string as a SPARQL angle-bracketed IRI via `oxrdf::NamedNode`.
///
/// # Panics
/// Panics if `iri_str` is not a valid IRI (this is a programming error — all
/// callers pass well-known literal URIs such as `urn:uuid:…`).
pub fn format_iri(iri_str: &str) -> String {
    let node = NamedNode::new(iri_str).expect("invalid IRI passed to format_iri — this is a bug");
    node.to_string()
}
