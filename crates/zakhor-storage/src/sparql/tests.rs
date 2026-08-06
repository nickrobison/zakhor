use super::*;

#[test]
fn test_ontology_construct_prefix() {
    let q = ontology_construct("?s ?p ?o .", "?s ?p ?o .");
    assert!(q.starts_with("PREFIX"));
    assert!(q.contains("CONSTRUCT {"));
    assert!(q.contains("WHERE {"));
}

#[test]
fn test_prefix_count() {
    let q = ontology_construct("?s ?p ?o .", "?s ?p ?o .");
    let prefix_count = q.lines().filter(|l| l.starts_with("PREFIX")).count();
    assert_eq!(
        prefix_count,
        PREFIX_LIST.len(),
        "all prefixes should be declared"
    );
}

#[test]
fn test_prefix_nie() {
    let q = ontology_construct("?s ?p ?o .", "?s ?p ?o .");
    assert!(q.contains("PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>"));
}

#[test]
fn test_prefix_rdf() {
    let q = ontology_construct("?s ?p ?o .", "?s ?p ?o .");
    assert!(q.contains("PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>"));
}

#[test]
fn test_literal_with_quotes_is_escaped() {
    let text = "hello \"world\"";
    let q = SparqlBuilder::insert_data("urn:uuid:x", text);
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
    // oxrdf escapes tab as \t inside a SPARQL short literal
    assert!(
        q.contains(r#""col1\tcol2""#),
        "tab must be escaped as \\t inside quoted literal: {}",
        q
    );
}

// -- injection tests -------------------------------------------------------

#[test]
fn test_injection_attack_is_safely_escaped() {
    let text = "x\"; DROP ALL; \"";
    let q = SparqlBuilder::insert_data("urn:uuid:inj", text);
    assert!(
        q.contains(r#""x\"; DROP ALL; \"""#),
        "quotes must be escaped inside literal: {}",
        q
    );
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
            "insert_data_raw",
            SparqlBuilder::insert_data_raw("?s ?p ?o ."),
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

#[test]
fn test_braces_balanced() {
    let q = ontology_construct("?s ?p ?o .", "?s ?p ?o .");
    let open = q.matches('{').count();
    let close = q.matches('}').count();
    assert_eq!(open, close, "unbalanced braces in {}", q);
}
