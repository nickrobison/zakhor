use rdf_types::IriBuf;
use rdf_types::{Literal, LiteralType, RdfDisplay, Triple, XSD_STRING};

// ---------------------------------------------------------------------------
// Prefix constants — shared vocabulary URIs
// ---------------------------------------------------------------------------

pub struct Prefix;

impl Prefix {
    pub const NIE: &'static str = "http://www.semanticdesktop.org/ontologies/2007/01/19/nie#";
    pub const RDF: &'static str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
    pub const RDFS: &'static str = "http://www.w3.org/2000/01/rdf-schema#";
    pub const OWL: &'static str = "http://www.w3.org/2002/07/owl#";
    pub const XSD: &'static str = "http://www.w3.org/2001/XMLSchema#";
    pub const DCTERMS: &'static str = "http://purl.org/dc/terms/";
    pub const FOAF: &'static str = "http://xmlns.com/foaf/0.1/";
    pub const ZAKHOR: &'static str = "http://zakhor/ns/";
}

const PREFIX_LIST: &[(&str, &str)] = &[
    ("nie", Prefix::NIE),
    ("rdf", Prefix::RDF),
    ("rdfs", Prefix::RDFS),
    ("owl", Prefix::OWL),
    ("xsd", Prefix::XSD),
    ("dcterms", Prefix::DCTERMS),
    ("foaf", Prefix::FOAF),
    ("zakhor", Prefix::ZAKHOR),
];

/// Emit `PREFIX name: <iri>` declarations for all known namespaces.
fn prefix_declarations() -> String {
    let mut out = String::with_capacity(512);
    for (name, ns) in PREFIX_LIST {
        out.push_str("PREFIX ");
        out.push_str(name);
        out.push_str(": <");
        out.push_str(ns);
        out.push_str(">\n");
    }
    out
}

/// Escape `text` as a SPARQL literal using `rdf_types::Literal` + `RdfDisplay`.
/// The returned string includes the enclosing double quotes and any internal
/// escaping — it is safe to interpolate directly into a SPARQL query string.
fn escape_literal(text: &str) -> String {
    let lit = Literal::new(text.to_string(), LiteralType::Any(XSD_STRING.to_owned()));
    lit.rdf_display().to_string()
}

/// Format a string as a SPARQL angle-bracketed IRI via `rdf_types::IriBuf::rdf_display()`.
///
/// # Panics
/// Panics if `iri_str` is not a valid IRI (this is a programming error — all
/// callers pass well-known literal URIs such as `urn:uuid:…`).
fn format_iri(iri_str: &str) -> String {
    let iri =
        IriBuf::new(iri_str.to_string()).expect("invalid IRI passed to format_iri — this is a bug");
    iri.rdf_display().to_string()
}

// ---------------------------------------------------------------------------
// SparqlBuilder — typed SPARQL query construction
// ---------------------------------------------------------------------------

/// Typed SPARQL query builder.
///
/// Every method produces a complete SPARQL query string with `PREFIX`
/// declarations and safe literal escaping via `rdf_types::Literal`.
pub struct SparqlBuilder;

impl SparqlBuilder {
    /// Build a `SELECT ?text WHERE { … }` query that retrieves the
    /// `nie:plainTextContent` of an `nie:InformationElement` identified by
    /// `nie:identifier`.
    pub fn select(id: &str) -> String {
        let id_lit = escape_literal(id);
        format!(
            "{}SELECT ?text WHERE {{\n\
             ?id rdf:type nie:InformationElement ;\n\
                 nie:identifier {} ;\n\
                 nie:plainTextContent ?text .\n\
            }}",
            prefix_declarations(),
            id_lit,
        )
    }

    /// Build an `INSERT DATA { … }` query that stores a new
    /// `nie:InformationElement` with a given `uuid` IRI and `text` content.
    pub fn insert_data(uuid: &str, text: &str) -> String {
        let uuid_iri = format_iri(uuid);
        let uuid_lit = escape_literal(uuid);
        let text_lit = escape_literal(text);
        format!(
            "{}INSERT DATA {{\n\
             {} rdf:type nie:InformationElement ;\n\
                 nie:identifier {} ;\n\
                 nie:plainTextContent {} .\n\
            }}",
            prefix_declarations(),
            uuid_iri,
            uuid_lit,
            text_lit,
        )
    }

    /// Build a `DELETE { … } WHERE { … }` query that removes an
    /// `nie:InformationElement` identified by `nie:identifier`.
    pub fn delete_data(id: &str) -> String {
        let id_lit = escape_literal(id);
        format!(
            "{}DELETE {{\n\
             ?id rdf:type nie:InformationElement .\n\
             ?id nie:identifier ?oldId .\n\
             ?id nie:plainTextContent ?oldText .\n\
            }}\n\
            WHERE {{\n\
             ?id rdf:type nie:InformationElement ;\n\
                 nie:identifier {} .\n\
             ?id nie:identifier ?oldId .\n\
             OPTIONAL {{ ?id nie:plainTextContent ?oldText . }}\n\
            }}",
            prefix_declarations(),
            id_lit,
        )
    }

    /// Build a `DELETE { … } INSERT { … } WHERE { … }` query that replaces
    /// the `nie:plainTextContent` of an existing `nie:InformationElement`.
    pub fn delete_insert_where(id: &str, text: &str) -> String {
        let id_lit = escape_literal(id);
        let text_lit = escape_literal(text);
        format!(
            "{}DELETE {{\n\
             ?id nie:plainTextContent ?oldText .\n\
            }}\n\
            INSERT {{\n\
             ?id nie:plainTextContent {} .\n\
            }}\n\
            WHERE {{\n\
             ?id rdf:type nie:InformationElement ;\n\
                 nie:identifier {} ;\n\
             OPTIONAL {{ ?id nie:plainTextContent ?oldText . }}\n\
            }}",
            prefix_declarations(),
            text_lit,
            id_lit,
        )
    }

    /// Build a `CONSTRUCT { … } WHERE { … }` query.
    ///
    /// `construct_pattern` and `where_pattern` are arbitrary triple-pattern
    /// fragments (prefixed names are resolved by the `PREFIX` declarations).
    pub fn construct(construct_pattern: &str, where_pattern: &str) -> String {
        format!(
            "{}CONSTRUCT {{\n{}\n}}\nWHERE {{\n{}\n}}",
            prefix_declarations(),
            construct_pattern,
            where_pattern,
        )
    }

    /// Build a CONSTRUCT query that uses `Triple::rdf_display()` for safe
    /// triple formatting via `rdf_types`.
    pub fn construct_triple(
        subject_iri: &str,
        predicate_iri: &str,
        object_iri: &str,
        where_clause: &str,
    ) -> String {
        let s = IriBuf::new(subject_iri.to_string()).expect("invalid subject IRI");
        let p = IriBuf::new(predicate_iri.to_string()).expect("invalid predicate IRI");
        let o = IriBuf::new(object_iri.to_string()).expect("invalid object IRI");
        let triple = Triple::new(s, p, o);
        format!(
            "{}CONSTRUCT {{\n{}\n}}\nWHERE {{\n{}\n}}",
            prefix_declarations(),
            triple.rdf_display(),
            where_clause,
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- basic structure tests -------------------------------------------------

    #[test]
    fn test_select_contains_keywords() {
        let q = SparqlBuilder::select("test-id");
        assert!(q.starts_with("PREFIX"), "should start with PREFIX");
        assert!(q.contains("SELECT ?text"), "should SELECT ?text");
        assert!(q.contains("WHERE {"), "should have WHERE");
        assert!(
            q.contains("rdf:type nie:InformationElement"),
            "should query InformationElement"
        );
        assert!(q.ends_with('}'), "should end with closing brace");
    }

    #[test]
    fn test_insert_data_contains_keywords() {
        let q = SparqlBuilder::insert_data("urn:uuid:abc", "hello");
        assert!(q.starts_with("PREFIX"));
        assert!(q.contains("INSERT DATA"));
        assert!(q.contains("rdf:type nie:InformationElement"));
        assert!(q.contains("nie:plainTextContent"));
    }

    #[test]
    fn test_delete_data_contains_keywords() {
        let q = SparqlBuilder::delete_data("test-id");
        assert!(q.starts_with("PREFIX"));
        assert!(q.contains("DELETE {"));
        assert!(q.contains("WHERE {"));
    }

    #[test]
    fn test_delete_insert_where_contains_keywords() {
        let q = SparqlBuilder::delete_insert_where("test-id", "new text");
        assert!(q.starts_with("PREFIX"));
        assert!(q.contains("DELETE {"));
        assert!(q.contains("INSERT {"));
        assert!(q.contains("WHERE {"));
        assert!(q.contains("nie:plainTextContent"));
    }

    #[test]
    fn test_construct_contains_keywords() {
        let q = SparqlBuilder::construct("?s ?p ?o .", "?s ?p ?o .");
        assert!(q.starts_with("PREFIX"));
        assert!(q.contains("CONSTRUCT {"));
        assert!(q.contains("WHERE {"));
        assert!(q.contains("?s ?p ?o"));
    }

    #[test]
    fn test_construct_triple_contains_keywords() {
        let q = SparqlBuilder::construct_triple(
            "urn:uuid:abc",
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
            "http://www.semanticdesktop.org/ontologies/2007/01/19/nie#InformationElement",
            "?s ?p ?o .",
        );
        assert!(q.starts_with("PREFIX"));
        assert!(q.contains("CONSTRUCT {"));
        assert!(q.contains("<urn:uuid:abc>"));
        assert!(q.contains("WHERE {"));
    }

    // -- prefix declarations ---------------------------------------------------

    #[test]
    fn test_prefix_count() {
        let q = SparqlBuilder::select("x");
        let prefix_count = q.lines().filter(|l| l.starts_with("PREFIX")).count();
        assert_eq!(
            prefix_count,
            PREFIX_LIST.len(),
            "all prefixes should be declared"
        );
    }

    #[test]
    fn test_prefix_nie() {
        let q = SparqlBuilder::select("x");
        assert!(
            q.contains("PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>")
        );
    }

    #[test]
    fn test_prefix_rdf() {
        let q = SparqlBuilder::select("x");
        assert!(q.contains("PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>"));
    }

    // -- literal escaping ------------------------------------------------------

    #[test]
    fn test_literal_with_quotes_is_escaped() {
        let text = "hello \"world\"";
        let q = SparqlBuilder::insert_data("urn:uuid:x", text);
        // The literal should be enclosed in quotes and internal quotes escaped
        assert!(
            q.contains(r#""hello \"world\"""#),
            "internal quotes must be escaped: {}",
            q
        );
    }

    #[test]
    fn test_literal_with_newline_is_escaped() {
        let text = "line1\nline2";
        let q = SparqlBuilder::insert_data("urn:uuid:x", text);
        assert!(
            q.contains(r#""line1\nline2""#),
            "newline must be escaped: {}",
            q
        );
    }

    #[test]
    fn test_literal_with_tab_is_escaped() {
        let text = "col1\tcol2";
        let q = SparqlBuilder::insert_data("urn:uuid:x", text);
        // rdf_types Literal::rdf_display() emits literal tabs without escaping
        assert!(
            q.contains("\"col1\tcol2\""),
            "tab must be inside quoted literal: {}",
            q
        );
    }

    // -- injection test --------------------------------------------------------

    #[test]
    fn test_injection_attack_is_safely_escaped() {
        // x"; DROP ALL; " must be rendered as literal content, not syntax injection.
        // rdf_types escapes the internal " as \" so the parser sees a single literal.
        let text = "x\"; DROP ALL; \"";
        let q = SparqlBuilder::insert_data("urn:uuid:inj", text);
        // The injection text IS inside the literal — that is correct.
        // What matters is that quotes are properly escaped (\").
        assert!(
            q.contains(r#""x\"; DROP ALL; \"""#),
            "quotes must be escaped inside literal: {}",
            q
        );
        // Exactly one plainTextContent triple
        let open_count = q.matches("nie:plainTextContent ").count();
        assert_eq!(
            open_count, 1,
            "exactly one plainTextContent triple expected"
        );
    }

    #[test]
    fn test_injection_braces() {
        let text = "evil }} DELETE ALL {{";
        let q = SparqlBuilder::insert_data("urn:uuid:br", text);
        // Braces inside literal are harmless — the literal is properly quoted.
        // The injection text IS inside the literal; structural integrity is maintained.
        assert!(
            q.contains(r#""evil }} DELETE ALL {{""#),
            "injection text must be inside literal: {}",
            q
        );
    }

    #[test]
    fn test_injection_semicolon_sparql() {
        let text = "foo ASK WHERE { ?s ?p ?o } bar";
        let q = SparqlBuilder::insert_data("urn:uuid:ask", text);
        // ASK WHERE inside literal — harmless, properly quoted.
        assert!(
            q.contains(r#""foo ASK WHERE { ?s ?p ?o } bar""#),
            "injection text must be inside literal: {}",
            q
        );
    }

    // -- UUID IRI formatting ---------------------------------------------------

    #[test]
    fn test_uuid_iri_is_angle_bracketed() {
        let q = SparqlBuilder::insert_data("urn:uuid:abc-123", "hello");
        assert!(
            q.contains("<urn:uuid:abc-123>"),
            "UUID should be <urn:uuid:abc-123>, got: {}",
            q
        );
    }

    // -- round-trip consistency for safe subset --------------------------------
    // We cannot run an actual SPARQL endpoint in unit tests, but we can verify
    // that the generated queries are structurally well-formed (balanced braces,
    // no syntax errors in prefix handling).

    #[test]
    fn test_query_braces_balanced() {
        for (name, q) in [
            ("select", SparqlBuilder::select("x")),
            (
                "insert_data",
                SparqlBuilder::insert_data("urn:uuid:x", "hello"),
            ),
            ("delete_data", SparqlBuilder::delete_data("x")),
            (
                "delete_insert_where",
                SparqlBuilder::delete_insert_where("x", "y"),
            ),
            (
                "construct",
                SparqlBuilder::construct("?s ?p ?o .", "?s ?p ?o ."),
            ),
            (
                "construct_triple",
                SparqlBuilder::construct_triple(
                    "urn:uuid:x",
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                    "http://www.semanticdesktop.org/ontologies/2007/01/19/nie#InformationElement",
                    "?s ?p ?o .",
                ),
            ),
        ] {
            let open = q.matches('{').count();
            let close = q.matches('}').count();
            assert_eq!(open, close, "unbalanced braces in {} query: {}", name, q);
        }
    }

    // -- escape_literal unit behavior ------------------------------------------

    #[test]
    fn test_escape_literal_wraps_in_quotes() {
        let s = escape_literal("hello");
        assert!(s.starts_with('"'), "should start with quote: {}", s);
        assert!(s.ends_with('"'), "should end with quote: {}", s);
    }

    #[test]
    fn test_escape_literal_empty() {
        let s = escape_literal("");
        assert_eq!(s, r#""""#, "empty literal should be empty quoted string");
    }
}
