use gio::Cancellable;
use rdf_types::{IriBuf, Literal, LiteralType, RdfDisplay, XSD_STRING};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracker::SparqlConnection;
use tracker::prelude::SparqlConnectionExtManual;

use crate::provenance::ProvenanceTracker;
use crate::sparql::Prefix;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// An entity reference associated with an observation.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct EntityRef {
    pub uri: String,
    pub label: String,
}

/// A relation between two entities.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct Relation {
    pub subject_uri: String,
    pub predicate_uri: String,
    pub object_uri: String,
    pub label: String,
}

/// Arguments for storing a complete observation.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct StoreObservationArgs {
    pub text: String,
    pub entities: Vec<EntityRef>,
    pub relations: Vec<Relation>,
}

/// Result of a successfully ingested observation.
#[derive(Clone, Debug)]
pub struct IngestResult {
    pub observation_uri: String,
    pub triple_count: usize,
}

// ---------------------------------------------------------------------------
// IngestionPipeline
// ---------------------------------------------------------------------------

/// Ingestion pipeline that persists observations as SPARQL triples
/// with named-graph provenance tracking.
pub struct IngestionPipeline {
    provenance: ProvenanceTracker,
}

impl IngestionPipeline {
    /// Creates a new ingestion pipeline with empty provenance.
    pub fn new() -> Self {
        Self {
            provenance: ProvenanceTracker::new(),
        }
    }

    /// Ingest a complete observation: creates the `nie:InformationElement`,
    /// entities, and relations in a single SPARQL transaction with named-graph
    /// provenance tracking.
    pub fn ingest(
        &mut self,
        conn: &SparqlConnection,
        args: StoreObservationArgs,
    ) -> Result<IngestResult, String> {
        let uuid_urn: String = tracker::functions::sparql_get_uuid_urn()
            .ok_or_else(|| "Failed to generate UUID for observation".to_string())?
            .to_string();

        let sparql = build_observation_sparql(&args, &uuid_urn);

        conn.update(&sparql, None::<&Cancellable>)
            .map_err(|e| format!("Failed to ingest observation: {}", e))?;

        // Track provenance locally
        let uuid_part = uuid_urn.strip_prefix("urn:uuid:").unwrap_or(&uuid_urn);
        let triples = collect_provenance_triples(&args, &uuid_urn);
        let triple_count = triples.len();
        self.provenance.add_observation(uuid_part, triples);

        Ok(IngestResult {
            observation_uri: uuid_urn,
            triple_count,
        })
    }

    /// Get the provenance tracker (for querying graph history).
    #[allow(dead_code)]
    pub fn provenance(&self) -> &ProvenanceTracker {
        &self.provenance
    }
}

impl Default for IngestionPipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// SPARQL query builder
// ---------------------------------------------------------------------------

/// Emit `PREFIX name: <iri>` declarations for all known namespaces.
fn prefix_declarations() -> String {
    let mut out = String::with_capacity(512);
    for (name, ns) in &[
        ("nie", Prefix::NIE),
        ("rdf", Prefix::RDF),
        ("rdfs", Prefix::RDFS),
        ("owl", Prefix::OWL),
        ("xsd", Prefix::XSD),
        ("dcterms", Prefix::DCTERMS),
        ("foaf", Prefix::FOAF),
        ("zakhor", Prefix::ZAKHOR),
    ] {
        out.push_str("PREFIX ");
        out.push_str(name);
        out.push_str(": <");
        out.push_str(ns);
        out.push_str(">\n");
    }
    out
}

/// Format a string as a SPARQL angle-bracketed IRI.
///
/// # Panics
/// Panics if `iri_str` is not a valid IRI (programming error — callers
/// pass well-known literal URIs such as `urn:uuid:…` or valid HTTP IRIs).
fn format_iri(iri_str: &str) -> String {
    let iri =
        IriBuf::new(iri_str.to_string()).expect("invalid IRI passed to format_iri — this is a bug");
    iri.rdf_display().to_string()
}

/// Escape `text` as a SPARQL literal using `rdf_types::Literal` + `RdfDisplay`.
/// The returned string includes the enclosing double quotes and any internal
/// escaping — it is safe to interpolate directly into a SPARQL query string.
fn escape_literal(text: &str) -> String {
    let lit = Literal::new(text.to_string(), LiteralType::Any(XSD_STRING.to_owned()));
    lit.rdf_display().to_string()
}

/// Build the full `INSERT DATA { … }` SPARQL query for an observation.
///
/// This function does **not** connect to Tracker — it only builds the SPARQL
/// string, making it testable in unit tests.
///
/// `uuid_urn` must be a `urn:uuid:…` string such as `urn:uuid:abc-123`.
fn build_observation_sparql(args: &StoreObservationArgs, uuid_urn: &str) -> String {
    let mut sparql = String::with_capacity(2048);

    // PREFIX declarations
    sparql.push_str(&prefix_declarations());

    // INSERT DATA header
    sparql.push_str("INSERT DATA {\n");

    // -- Base InformationElement ------------------------------------------------
    let uuid_iri = format_iri(uuid_urn);
    let uuid_lit = escape_literal(uuid_urn);
    let text_lit = escape_literal(&args.text);

    sparql.push_str(&format!(
        "  {} rdf:type nie:InformationElement ;\n",
        uuid_iri
    ));
    sparql.push_str(&format!("    nie:identifier {} ;\n", uuid_lit));
    sparql.push_str(&format!("    nie:plainTextContent {} .\n", text_lit));

    // -- Entity triples ---------------------------------------------------------
    for entity in &args.entities {
        let entity_iri = format_iri(&entity.uri);
        let label_lit = escape_literal(&entity.label);

        sparql.push_str(&format!(
            "  {} zakhor:hasEntity {} .\n",
            uuid_iri, entity_iri,
        ));
        sparql.push_str(&format!(
            "  {} rdf:type zakhor:Entity ; rdfs:label {} .\n",
            entity_iri, label_lit,
        ));
    }

    // -- Relation triples -------------------------------------------------------
    for relation in &args.relations {
        let subj_iri = format_iri(&relation.subject_uri);
        let pred_iri = format_iri(&relation.predicate_uri);
        let obj_iri = format_iri(&relation.object_uri);

        sparql.push_str(&format!("  {} {} {} .\n", subj_iri, pred_iri, obj_iri,));
    }

    sparql.push_str("}\n");
    sparql
}

// ---------------------------------------------------------------------------
// Provenance helpers
// ---------------------------------------------------------------------------

/// Collect all triples that were inserted into the SPARQL store so they can
/// be tracked locally by `ProvenanceTracker`.
///
/// Each triple is a `(subject, predicate, object)` tuple of full IRIs.
fn collect_provenance_triples(
    args: &StoreObservationArgs,
    uuid_urn: &str,
) -> Vec<(String, String, String)> {
    let mut triples = Vec::with_capacity(3 + args.entities.len() * 3 + args.relations.len());

    // Information element triples
    triples.push((
        uuid_urn.to_string(),
        format!("{}type", Prefix::RDF),
        format!("{}InformationElement", Prefix::NIE),
    ));
    triples.push((
        uuid_urn.to_string(),
        format!("{}identifier", Prefix::NIE),
        uuid_urn.to_string(),
    ));
    triples.push((
        uuid_urn.to_string(),
        format!("{}plainTextContent", Prefix::NIE),
        args.text.clone(),
    ));

    // Entity triples
    for entity in &args.entities {
        triples.push((
            uuid_urn.to_string(),
            format!("{}hasEntity", Prefix::ZAKHOR),
            entity.uri.clone(),
        ));
        triples.push((
            entity.uri.clone(),
            format!("{}type", Prefix::RDF),
            format!("{}Entity", Prefix::ZAKHOR),
        ));
        triples.push((
            entity.uri.clone(),
            format!("{}label", Prefix::RDFS),
            entity.label.clone(),
        ));
    }

    // Relation triples
    for relation in &args.relations {
        triples.push((
            relation.subject_uri.clone(),
            relation.predicate_uri.clone(),
            relation.object_uri.clone(),
        ));
    }

    triples
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Pipeline lifecycle ---------------------------------------------------

    #[test]
    fn test_pipeline_new_is_empty() {
        let pipeline = IngestionPipeline::new();
        assert!(pipeline.provenance().all_observations().is_empty());
        assert!(!pipeline.provenance().contains_observation("any-uuid"));
    }

    // -- SPARQL query building ------------------------------------------------

    #[test]
    fn test_build_observation_sparql_contains_all_parts() {
        let args = StoreObservationArgs {
            text: "test observation text".into(),
            entities: vec![EntityRef {
                uri: "http://example.com/entity1".into(),
                label: "Entity One".into(),
            }],
            relations: vec![Relation {
                subject_uri: "http://example.com/subj1".into(),
                predicate_uri: "http://example.com/pred1".into(),
                object_uri: "http://example.com/obj1".into(),
                label: "related".into(),
            }],
        };

        let sparql = build_observation_sparql(&args, "urn:uuid:test-uuid-1");

        // Structural checks
        assert!(sparql.starts_with("PREFIX"), "should start with PREFIX");
        assert!(
            sparql.contains("INSERT DATA {"),
            "should contain INSERT DATA"
        );
        assert!(sparql.ends_with("}\n"), "should end with closing brace");

        // InformationElement
        assert!(
            sparql.contains("rdf:type nie:InformationElement"),
            "should type as InformationElement"
        );
        assert!(sparql.contains("nie:identifier"), "should have identifier");
        assert!(
            sparql.contains("nie:plainTextContent"),
            "should have plainTextContent"
        );
        assert!(
            sparql.contains("test observation text"),
            "should contain text content"
        );

        // Entity
        assert!(sparql.contains("zakhor:hasEntity"), "should link entities");
        assert!(sparql.contains("zakhor:Entity"), "should type as Entity");
        assert!(sparql.contains("rdfs:label"), "should label entity");
        assert!(sparql.contains("Entity One"), "should contain entity label");

        // Relation
        assert!(
            sparql.contains("<http://example.com/subj1>"),
            "should contain subject IRI"
        );
        assert!(
            sparql.contains("<http://example.com/pred1>"),
            "should contain predicate IRI"
        );
        assert!(
            sparql.contains("<http://example.com/obj1>"),
            "should contain object IRI"
        );

        // Balanced braces
        let opens = sparql.matches('{').count();
        let closes = sparql.matches('}').count();
        assert_eq!(
            opens, closes,
            "braces should be balanced: {} vs {}",
            opens, closes
        );
    }

    #[test]
    fn test_build_observation_with_entities() {
        let args = StoreObservationArgs {
            text: "text with entities".into(),
            entities: vec![
                EntityRef {
                    uri: "http://example.com/e1".into(),
                    label: "Entity 1".into(),
                },
                EntityRef {
                    uri: "http://example.com/e2".into(),
                    label: "Entity 2".into(),
                },
            ],
            relations: vec![],
        };

        let sparql = build_observation_sparql(&args, "urn:uuid:entity-test");

        // Both entities should appear
        assert!(
            sparql.contains("<http://example.com/e1>"),
            "first entity IRI"
        );
        assert!(
            sparql.contains("<http://example.com/e2>"),
            "second entity IRI"
        );
        assert!(sparql.contains("Entity 1"), "first entity label");
        assert!(sparql.contains("Entity 2"), "second entity label");

        // Each entity produces 2 triples (hasEntity + type+label), plus base 3
        let has_entity_count = sparql.matches("zakhor:hasEntity").count();
        assert_eq!(has_entity_count, 2, "should have 2 hasEntity links");

        let entity_type_count = sparql.matches("zakhor:Entity").count();
        assert_eq!(
            entity_type_count, 2,
            "should have 2 Entity type declarations"
        );
    }

    #[test]
    fn test_build_observation_with_relations() {
        let args = StoreObservationArgs {
            text: "text with relations".into(),
            entities: vec![],
            relations: vec![
                Relation {
                    subject_uri: "http://example.com/s1".into(),
                    predicate_uri: "http://example.com/p1".into(),
                    object_uri: "http://example.com/o1".into(),
                    label: "relates to".into(),
                },
                Relation {
                    subject_uri: "http://example.com/s2".into(),
                    predicate_uri: "http://example.com/p2".into(),
                    object_uri: "http://example.com/o2".into(),
                    label: "depends on".into(),
                },
            ],
        };

        let sparql = build_observation_sparql(&args, "urn:uuid:rel-test");

        // Both relation triples should appear
        assert!(
            sparql.contains(
                "<http://example.com/s1> <http://example.com/p1> <http://example.com/o1>"
            )
        );
        assert!(
            sparql.contains(
                "<http://example.com/s2> <http://example.com/p2> <http://example.com/o2>"
            )
        );
    }

    #[test]
    fn test_build_observation_with_no_entities_or_relations() {
        let args = StoreObservationArgs {
            text: "bare text".into(),
            entities: vec![],
            relations: vec![],
        };

        let sparql = build_observation_sparql(&args, "urn:uuid:bare");

        // Should still have the base InformationElement
        assert!(sparql.contains("rdf:type nie:InformationElement"));
        assert!(sparql.contains("nie:plainTextContent"));
        assert!(sparql.contains("bare text"));
        assert!(sparql.contains("<urn:uuid:bare>"));

        // Should NOT have entity or relation triples
        assert!(!sparql.contains("zakhor:hasEntity"));
        assert!(!sparql.contains("zakhor:Entity"));
    }

    // -- Data structure construction -----------------------------------------

    #[test]
    fn test_store_observation_args_struct() {
        let args = StoreObservationArgs {
            text: "hello".into(),
            entities: vec![EntityRef {
                uri: "http://example.com/e".into(),
                label: "E".into(),
            }],
            relations: vec![Relation {
                subject_uri: "http://example.com/s".into(),
                predicate_uri: "http://example.com/p".into(),
                object_uri: "http://example.com/o".into(),
                label: "r".into(),
            }],
        };
        assert_eq!(args.text, "hello");
        assert_eq!(args.entities.len(), 1);
        assert_eq!(args.entities[0].uri, "http://example.com/e");
        assert_eq!(args.relations.len(), 1);
        assert_eq!(args.relations[0].label, "r");
    }

    #[test]
    fn test_entity_ref_debug_and_clone() {
        let e1 = EntityRef {
            uri: "http://example.com/e1".into(),
            label: "Entity One".into(),
        };
        let e2 = e1.clone();
        assert_eq!(e1.uri, e2.uri);
        assert_eq!(e1.label, e2.label);
        let debug = format!("{:?}", e1);
        assert!(debug.contains("EntityRef"));
        assert!(debug.contains("http://example.com/e1"));
    }

    #[test]
    fn test_relation_debug_and_clone() {
        let r1 = Relation {
            subject_uri: "http://example.com/s".into(),
            predicate_uri: "http://example.com/p".into(),
            object_uri: "http://example.com/o".into(),
            label: "label".into(),
        };
        let r2 = r1.clone();
        assert_eq!(r1.subject_uri, r2.subject_uri);
        assert_eq!(r1.label, r2.label);
        let debug = format!("{:?}", r1);
        assert!(debug.contains("Relation"));
    }

    // -- Provenance triple collection ----------------------------------------

    #[test]
    fn test_collect_provenance_triples_basic() {
        let args = StoreObservationArgs {
            text: "test".into(),
            entities: vec![EntityRef {
                uri: "http://example.com/e".into(),
                label: "E".into(),
            }],
            relations: vec![Relation {
                subject_uri: "http://example.com/s".into(),
                predicate_uri: "http://example.com/p".into(),
                object_uri: "http://example.com/o".into(),
                label: "r".into(),
            }],
        };

        let triples = collect_provenance_triples(&args, "urn:uuid:test-coll");

        // 3 base + 3 entity + 1 relation = 7
        assert_eq!(triples.len(), 7);

        // Check base triples
        assert!(triples.contains(&(
            "urn:uuid:test-coll".to_string(),
            format!("{}type", Prefix::RDF),
            format!("{}InformationElement", Prefix::NIE),
        )));
        assert!(triples.contains(&(
            "urn:uuid:test-coll".to_string(),
            format!("{}plainTextContent", Prefix::NIE),
            "test".to_string(),
        )));

        // Check entity triples
        assert!(triples.contains(&(
            "urn:uuid:test-coll".to_string(),
            format!("{}hasEntity", Prefix::ZAKHOR),
            "http://example.com/e".to_string(),
        )));

        // Check relation triple
        assert!(triples.contains(&(
            "http://example.com/s".to_string(),
            "http://example.com/p".to_string(),
            "http://example.com/o".to_string(),
        )));
    }

    #[test]
    fn test_collect_provenance_triples_empty_entities_relations() {
        let args = StoreObservationArgs {
            text: "bare".into(),
            entities: vec![],
            relations: vec![],
        };

        let triples = collect_provenance_triples(&args, "urn:uuid:bare-coll");
        // Only the 3 base InformationElement triples
        assert_eq!(triples.len(), 3);
    }

    // -- Prefix declaration helper -------------------------------------------

    #[test]
    fn test_prefix_declarations_are_complete() {
        let decls = prefix_declarations();
        assert!(decls.contains("PREFIX nie:"));
        assert!(decls.contains("PREFIX rdf:"));
        assert!(decls.contains("PREFIX rdfs:"));
        assert!(decls.contains("PREFIX owl:"));
        assert!(decls.contains("PREFIX xsd:"));
        assert!(decls.contains("PREFIX dcterms:"));
        assert!(decls.contains("PREFIX foaf:"));
        assert!(decls.contains("PREFIX zakhor:"));
    }
}
