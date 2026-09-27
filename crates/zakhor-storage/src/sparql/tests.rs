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
    let q = SparqlBuilder::insert_data("urn:uuid:x", text).expect("urn:uuid: IRI is valid");
    assert!(
        q.contains(r#""hello \"world\"""#),
        "internal quotes must be escaped: {}",
        q
    );
}

#[test]
fn test_literal_with_newline_is_escaped() {
    let text = "line1\nline2";
    let q = SparqlBuilder::insert_data("urn:uuid:x", text).expect("urn:uuid: IRI is valid");
    assert!(
        q.contains(r#""line1\nline2""#),
        "newline must be escaped: {}",
        q
    );
}

#[test]
fn test_literal_with_tab_is_escaped() {
    let text = "col1\tcol2";
    let q = SparqlBuilder::insert_data("urn:uuid:x", text).expect("urn:uuid: IRI is valid");
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
    let q = SparqlBuilder::insert_data("urn:uuid:inj", text).expect("urn:uuid: IRI is valid");
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
    let q = SparqlBuilder::insert_data("urn:uuid:br", text).expect("urn:uuid: IRI is valid");
    assert!(
        q.contains(r#""evil }} DELETE ALL {{""#),
        "injection text must be inside literal: {}",
        q
    );
}

#[test]
fn test_injection_semicolon_sparql() {
    let text = "foo ASK WHERE { ?s ?p ?o } bar";
    let q = SparqlBuilder::insert_data("urn:uuid:ask", text).expect("urn:uuid: IRI is valid");
    assert!(
        q.contains(r#""foo ASK WHERE { ?s ?p ?o } bar""#),
        "injection text must be inside literal: {}",
        q
    );
}

// -- UUID IRI formatting ---------------------------------------------------

#[test]
fn test_uuid_iri_is_angle_bracketed() {
    let q =
        SparqlBuilder::insert_data("urn:uuid:abc-123", "hello").expect("urn:uuid: IRI is valid");
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
            SparqlBuilder::insert_data("urn:uuid:x", "hello").expect("urn:uuid: IRI is valid"),
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

// -- URI validation and injection ------------------------------------------
//
// The literal tests above prove text cannot break out of a quoted literal.
// These prove the same for the other interpolation site: a URI, which is
// framed by angle brackets instead of quotes and so needs different handling.

/// Inputs that must never be accepted as an address.
fn hostile_uris() -> Vec<&'static str> {
    vec![
        "",
        "not-a-uri",
        "http://example.com/a b",
        "http://example.com/a\n} INSERT DATA { <x> <y> <z>",
        "http://example.com/a} . } INSERT DATA { <x> <y> <z",
        "http://example.com/a\" . \"b",
        "http://example.com/a\\b",
        "//example.com/no-scheme",
        "   ",
    ]
}

#[test]
fn test_format_iri_rejects_hostile_input() {
    for uri in hostile_uris() {
        assert!(
            format_iri(uri).is_err(),
            "must reject {uri:?} rather than interpolate it"
        );
    }
}

#[test]
fn test_format_iri_accepts_real_iris() {
    for uri in [
        "http://zakhor/ns/project/x",
        "urn:uuid:0e6d96e4-cc78-43f0-ac2f-fda9df18461d",
        "https://example.com/a%20b",
    ] {
        let formatted = format_iri(uri).unwrap_or_else(|e| panic!("{uri:?} should be valid: {e}"));
        assert_eq!(
            formatted,
            format!("<{uri}>"),
            "IRI must be re-wrapped in <>"
        );
    }
}

/// A NUL byte is the important case: it reached the storage layer as a panic
/// inside the GString conversion rather than as a validation error.
#[test]
fn test_format_iri_rejects_interior_nul() {
    assert!(format_iri("http://example.com/a\0b").is_err());
}

/// Whatever the input, the formatted output must contain exactly one IRI
/// reference — never a second one, which is what a structural break looks like.
#[test]
fn test_format_iri_output_always_has_one_iri_reference() {
    for uri in hostile_uris() {
        if let Ok(formatted) = format_iri(uri) {
            assert_eq!(
                formatted.matches('<').count(),
                1,
                "{uri:?} produced more than one IRI reference: {formatted}"
            );
        }
    }
}

#[test]
fn test_validate_iri_agrees_with_format_iri() {
    for uri in hostile_uris() {
        assert_eq!(
            validate_iri(uri).is_ok(),
            format_iri(uri).is_ok(),
            "validate_iri and format_iri must agree on {uri:?}"
        );
    }
}

/// The old behaviour stripped angle brackets, which silently accepted framed
/// input. It must now be rejected outright.
#[test]
fn test_bracketed_input_is_rejected_not_stripped() {
    assert!(format_iri("<http://example.com/a>").is_err());
}
