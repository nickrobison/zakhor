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
    pub const MEMORY: &'static str = "http://zakhor/ns/";
    pub const PROV: &'static str = "http://www.w3.org/ns/prov#";
    pub const PROV_WAS_DERIVED_FROM: &'static str = "http://www.w3.org/ns/prov#wasDerivedFrom";
}

pub(crate) const PREFIX_LIST: &[(&str, &str)] = &[
    ("nie", Prefix::NIE),
    ("rdf", Prefix::RDF),
    ("rdfs", Prefix::RDFS),
    ("owl", Prefix::OWL),
    ("xsd", Prefix::XSD),
    ("dcterms", Prefix::DCTERMS),
    ("foaf", Prefix::FOAF),
    ("zakhor", Prefix::ZAKHOR),
    ("memory", Prefix::MEMORY),
    ("prov", Prefix::PROV),
];

pub fn prefix_declarations() -> String {
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
