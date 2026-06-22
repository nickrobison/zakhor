use gio::Cancellable;
use rdf_types::{IriBuf, Literal, LiteralType, RdfDisplay, XSD_STRING};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracker::SparqlConnection;
use tracker::prelude::SparqlConnectionExtManual;

use crate::entity_resolver::EntityResolver;
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

/// Error type for ingestion pipeline stages.
#[derive(Debug)]
pub enum IngestionError {
    Validation(String),
    Resolution(String),
    Build(String),
    Persist(String),
    Sync(String),
}

impl std::fmt::Display for IngestionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IngestionError::Validation(msg) => write!(f, "validation: {}", msg),
            IngestionError::Resolution(msg) => write!(f, "resolution: {}", msg),
            IngestionError::Build(msg) => write!(f, "build: {}", msg),
            IngestionError::Persist(msg) => write!(f, "persist: {}", msg),
            IngestionError::Sync(msg) => write!(f, "sync: {}", msg),
        }
    }
}

impl std::error::Error for IngestionError {}

// ---------------------------------------------------------------------------
// 5-Stage IngestionPipeline
// ---------------------------------------------------------------------------

/// 5-stage ingestion pipeline for persisting observations.
///
/// Stages:
/// 1. **Validate** — Check that input args are well-formed.
/// 2. **Resolve** — Resolve entity labels to canonical URIs (skip if no resolver).
/// 3. **Build** — Construct SPARQL INSERT DATA + collect provenance triples.
/// 4. **Persist** — Execute SPARQL update against the triplestore.
/// 5. **Track** — Track provenance in-memory; optionally sync to search indexes.
pub struct IngestionPipeline {
    provenance: ProvenanceTracker,
    entity_resolver: Option<Arc<EntityResolver>>,
}

impl IngestionPipeline {
    pub fn new() -> Self {
        Self {
            provenance: ProvenanceTracker::new(),
            entity_resolver: None,
        }
    }

    /// Create a pipeline with an optional entity resolver.
    pub fn with_resolver(resolver: Option<Arc<EntityResolver>>) -> Self {
        Self {
            provenance: ProvenanceTracker::new(),
            entity_resolver: resolver,
        }
    }

    /// Run the full 5-stage ingestion pipeline.
    pub fn ingest(
        &mut self,
        conn: &SparqlConnection,
        args: StoreObservationArgs,
    ) -> Result<IngestResult, IngestionError> {
        // Stage 1: Validate
        self.validate(&args)?;

        // Stage 2: Resolve (mutate args in place if resolver is available)
        let mut args = args;
        if self.entity_resolver.is_some() {
            self.resolve_entities(&mut args)?;
        }

        // Stage 3: Build
        let uuid_urn: String = tracker::functions::sparql_get_uuid_urn()
            .ok_or_else(|| IngestionError::Build("Failed to generate UUID".to_string()))?
            .to_string();
        let (sparql, provenance_triples) = self.build_triples(&args, &uuid_urn);

        // Stage 4: Persist
        self.persist(conn, &sparql)?;

        // Stage 5: Track
        let triple_count = provenance_triples.len();
        let uuid_part = uuid_urn.strip_prefix("urn:uuid:").unwrap_or(&uuid_urn);
        self.provenance
            .add_observation(uuid_part, provenance_triples);

        Ok(IngestResult {
            observation_uri: uuid_urn,
            triple_count,
        })
    }

    /// Convenience: ingest + flush + return result.
    /// Flushes the in-memory provenance tracker to the SPARQL store.
    pub fn ingest_and_flush(
        &mut self,
        conn: &SparqlConnection,
        args: StoreObservationArgs,
    ) -> Result<IngestResult, IngestionError> {
        let result = self.ingest(conn, args)?;
        self.provenance
            .flush_to_sparql(conn)
            .map_err(|e| IngestionError::Persist(format!("flush failed: {}", e)))?;
        Ok(result)
    }

    /// Get the provenance tracker (for querying graph history).
    pub fn provenance(&self) -> &ProvenanceTracker {
        &self.provenance
    }

    // -----------------------------------------------------------------------
    // Stage implementations
    // -----------------------------------------------------------------------

    /// Stage 1: Validate input args.
    fn validate(&self, args: &StoreObservationArgs) -> Result<(), IngestionError> {
        if args.text.trim().is_empty() {
            return Err(IngestionError::Validation(
                "observation text must not be empty".to_string(),
            ));
        }
        for entity in &args.entities {
            if entity.uri.trim().is_empty() {
                return Err(IngestionError::Validation(
                    "entity URI must not be empty".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Stage 2: Resolve entity labels using the entity resolver.
    fn resolve_entities(&self, args: &mut StoreObservationArgs) -> Result<(), IngestionError> {
        let resolver = self.entity_resolver.as_ref().ok_or_else(|| {
            IngestionError::Resolution("entity resolver not configured".to_string())
        })?;

        for entity in &mut args.entities {
            if !entity.label.starts_with("http://") && !entity.label.starts_with("urn:") {
                let result = resolver.resolve(&entity.label);
                if let Some(ref uri) = result.resolved_uri {
                    entity.uri = uri.clone();
                }
            }
        }
        Ok(())
    }

    /// Stage 3: Build SPARQL query and collect provenance triples.
    fn build_triples(
        &self,
        args: &StoreObservationArgs,
        uuid_urn: &str,
    ) -> (String, Vec<(String, String, String)>) {
        let sparql = build_observation_sparql(args, uuid_urn);
        let triples = collect_provenance_triples(args, uuid_urn);
        (sparql, triples)
    }

    /// Stage 4: Persist to SPARQL triplestore.
    fn persist(&self, conn: &SparqlConnection, sparql: &str) -> Result<(), IngestionError> {
        conn.update(sparql, None::<&Cancellable>)
            .map_err(|e| IngestionError::Persist(format!("SPARQL update failed: {}", e)))
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

pub fn prefix_declarations() -> String {
    let mut out = String::with_capacity(512);
    for (name, ns) in &[
        ("nie", Prefix::NIE),
        ("rdf", Prefix::RDF),
        ("rdfs", Prefix::RDFS),
        ("owl", Prefix::OWL),
        ("xsd", Prefix::XSD),
        ("dcterms", Prefix::DCTERMS),
        ("foaf", Prefix::FOAF),
        ("prov", Prefix::PROV),
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

pub fn format_iri(iri_str: &str) -> String {
    let iri =
        IriBuf::new(iri_str.to_string()).expect("invalid IRI passed to format_iri — this is a bug");
    iri.rdf_display().to_string()
}

pub fn escape_literal(text: &str) -> String {
    let lit = Literal::new(text.to_string(), LiteralType::Any(XSD_STRING.to_owned()));
    lit.rdf_display().to_string()
}

/// Build the full `INSERT DATA { … }` SPARQL query for an observation.
///
/// `uuid_urn` must be a `urn:uuid:…` string such as `urn:uuid:abc-123`.
pub fn build_observation_sparql(args: &StoreObservationArgs, uuid_urn: &str) -> String {
    let mut sparql = String::with_capacity(2048);
    sparql.push_str(&prefix_declarations());
    sparql.push_str("INSERT DATA {\n");

    let uuid_iri = format_iri(uuid_urn);
    let uuid_lit = escape_literal(uuid_urn);
    let text_lit = escape_literal(&args.text);

    sparql.push_str(&format!(
        "  {} rdf:type nie:InformationElement ;\n",
        uuid_iri
    ));
    sparql.push_str(&format!("    nie:identifier {} ;\n", uuid_lit));
    sparql.push_str(&format!("    nie:plainTextContent {} .\n", text_lit));

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

/// Collect all triples inserted into the SPARQL store for local provenance tracking.
pub fn collect_provenance_triples(
    args: &StoreObservationArgs,
    uuid_urn: &str,
) -> Vec<(String, String, String)> {
    let mut triples = Vec::with_capacity(3 + args.entities.len() * 3 + args.relations.len());

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
// Errors
// ---------------------------------------------------------------------------

impl From<IngestionError> for String {
    fn from(e: IngestionError) -> String {
        e.to_string()
    }
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

    #[test]
    fn test_pipeline_without_resolver_does_not_crash() {
        let pipeline = IngestionPipeline::new();
        // Just verify it was created — cannot access private fields
        let _ = pipeline;
    }

    // -- Validation stage ----------------------------------------------------

    #[test]
    fn test_validate_rejects_empty_text() {
        let pipeline = IngestionPipeline::new();
        let args = StoreObservationArgs {
            text: "".into(),
            entities: vec![],
            relations: vec![],
        };
        let result = pipeline.validate(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_rejects_empty_entity_uri() {
        let pipeline = IngestionPipeline::new();
        let args = StoreObservationArgs {
            text: "some text".into(),
            entities: vec![EntityRef {
                uri: "".into(),
                label: "bad".into(),
            }],
            relations: vec![],
        };
        let result = pipeline.validate(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_accepts_valid_input() {
        let pipeline = IngestionPipeline::new();
        let args = StoreObservationArgs {
            text: "valid text".into(),
            entities: vec![EntityRef {
                uri: "http://example.com/e".into(),
                label: "E".into(),
            }],
            relations: vec![],
        };
        assert!(pipeline.validate(&args).is_ok());
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
        assert!(sparql.starts_with("PREFIX"), "should start with PREFIX");
        assert!(
            sparql.contains("INSERT DATA {"),
            "should contain INSERT DATA"
        );
        assert!(sparql.ends_with("}\n"), "should end with closing brace");
        assert!(sparql.contains("rdf:type nie:InformationElement"));
        assert!(sparql.contains("nie:identifier"));
        assert!(sparql.contains("nie:plainTextContent"));
        assert!(sparql.contains("test observation text"));
        assert!(sparql.contains("zakhor:hasEntity"));
        assert!(sparql.contains("zakhor:Entity"));
        assert!(sparql.contains("rdfs:label"));
        assert!(sparql.contains("Entity One"));
        assert!(sparql.contains("<http://example.com/subj1>"));
        assert!(sparql.contains("<http://example.com/pred1>"));
        assert!(sparql.contains("<http://example.com/obj1>"));
        let opens = sparql.matches('{').count();
        let closes = sparql.matches('}').count();
        assert_eq!(opens, closes, "braces should be balanced");
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
        assert!(sparql.contains("<http://example.com/e1>"));
        assert!(sparql.contains("<http://example.com/e2>"));
        assert!(sparql.contains("Entity 1"));
        assert!(sparql.contains("Entity 2"));
        assert_eq!(sparql.matches("zakhor:hasEntity").count(), 2);
        assert_eq!(sparql.matches("zakhor:Entity").count(), 2);
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
        assert!(sparql.contains("rdf:type nie:InformationElement"));
        assert!(sparql.contains("nie:plainTextContent"));
        assert!(sparql.contains("bare text"));
        assert!(sparql.contains("<urn:uuid:bare>"));
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
        assert_eq!(args.relations.len(), 1);
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
        assert_eq!(triples.len(), 7);
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
    }

    #[test]
    fn test_collect_provenance_triples_empty_entities_relations() {
        let args = StoreObservationArgs {
            text: "bare".into(),
            entities: vec![],
            relations: vec![],
        };
        let triples = collect_provenance_triples(&args, "urn:uuid:bare-coll");
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

    // -- IngestionError ------------------------------------------------------

    #[test]
    fn test_ingestion_error_display() {
        let e = IngestionError::Validation("bad input".into());
        let msg = format!("{}", e);
        assert!(msg.contains("validation: bad input"), "msg: {}", msg);
    }

    #[test]
    fn test_ingestion_error_from_string() {
        let e: String = IngestionError::Persist("disk full".into()).into();
        assert_eq!(e, "persist: disk full");
    }
}
