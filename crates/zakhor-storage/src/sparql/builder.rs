use oxrdf::{NamedNode, Triple};

use super::escape::{escape_literal, format_iri};
use super::prefix::{prefix_declarations, Prefix};

/// Typed SPARQL query builder.
///
/// Every method produces a complete SPARQL query string with `PREFIX`
/// declarations and safe literal escaping via `oxrdf::Literal`.
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

    /// Build a CONSTRUCT query that uses `Triple::fmt::Display` for safe
    /// triple formatting via `oxrdf`.
    pub fn construct_triple(
        subject_iri: &str,
        predicate_iri: &str,
        object_iri: &str,
        where_clause: &str,
    ) -> String {
        let s = NamedNode::new(subject_iri.to_string()).expect("invalid subject IRI");
        let p = NamedNode::new(predicate_iri.to_string()).expect("invalid predicate IRI");
        let o = NamedNode::new(object_iri.to_string()).expect("invalid object IRI");
        let triple = Triple::new(s, p, o);
        format!(
            "{}CONSTRUCT {{\n{}\n}}\nWHERE {{\n{}\n}}",
            prefix_declarations(),
            triple,
            where_clause,
        )
    }

    /// Build an `INSERT DATA { … }` query with arbitrary triple content.
    ///
    /// `triples` is a raw triple-pattern fragment (prefixed names are resolved
    /// by the `PREFIX` declarations emitted automatically).
    pub fn insert_data_raw(triples: &str) -> String {
        format!("{}INSERT DATA {{\n{}\n}}", prefix_declarations(), triples,)
    }

    /// Build a SELECT query that returns all triples in a specific named graph.
    ///
    /// Generates:
    /// ```sparql
    /// PREFIX ...
    /// SELECT ?s ?p ?o WHERE {
    ///   GRAPH <http://zakhor/ns/graph/{uuid}> { ?s ?p ?o }
    /// }
    /// ```
    pub fn select_graph(observation_uuid: &str) -> String {
        let graph_iri = format!("{}graph/{}", Prefix::ZAKHOR, observation_uuid);
        format!(
            "{}SELECT ?s ?p ?o WHERE {{\n\
             GRAPH <{}> {{ ?s ?p ?o }}\n\
            }}",
            prefix_declarations(),
            graph_iri,
        )
    }
}
