#![allow(dead_code)]

use iref::Iri;
use static_iref::iri;

pub use crate::sparql::Prefix;
pub use crate::sparql::SparqlBuilder;

/// Additional namespace constants (beyond those in Prefix)
pub const NFO: &str = "http://www.semanticdesktop.org/ontologies/2007/03/22/nfo#";
pub const NAO: &str = "http://www.semanticdesktop.org/ontologies/2007/08/15/nao#";
pub const SKOS: &str = "http://www.w3.org/2004/02/skos/core#";
pub const NRL: &str = "http://tracker.api.gnome.org/ontology/v3/nrl#";

/// All prefix entries for SPARQL queries (adds NFO, NAO, SKOS to basic set from sparql.rs)
pub const EXTRA_PREFIXES: &[(&str, &str)] = &[("nfo", NFO), ("nao", NAO), ("skos", SKOS)];

// IRI constructor functions — compile-time parsed static IRIs:

pub fn entity_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Entity")
}

pub fn decision_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Decision")
}

pub fn project_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Project")
}

pub fn issue_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Issue")
}

pub fn constraint_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Constraint")
}

pub fn observation_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Observation")
}

pub fn has_entity_iri() -> &'static Iri {
    iri!("http://zakhor/ns/hasEntity")
}

pub fn has_relation_iri() -> &'static Iri {
    iri!("http://zakhor/ns/hasRelation")
}

pub fn provenance_graph_iri() -> &'static Iri {
    iri!("http://zakhor/ns/provenanceGraph")
}

pub fn decision_context_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionContext")
}
pub fn decision_outcome_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionOutcome")
}
pub fn decision_alternative_iri() -> &'static Iri {
    iri!("http://zakhor/ns/alternative")
}
pub fn decision_rationale_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionRationale")
}

// --- v2 vocabulary additions (from vocab.rs) ---

pub fn conflicts_with_iri() -> &'static Iri {
    iri!("http://zakhor/ns/conflictsWith")
}

pub fn depends_on_iri() -> &'static Iri {
    iri!("http://zakhor/ns/dependsOn")
}

pub fn supersedes_iri() -> &'static Iri {
    iri!("http://zakhor/ns/supersedes")
}

pub fn evidence_for_iri() -> &'static Iri {
    iri!("http://zakhor/ns/evidenceFor")
}

pub fn belongs_to_project_iri() -> &'static Iri {
    iri!("http://zakhor/ns/belongsToProject")
}

pub fn code_location_iri() -> &'static Iri {
    iri!("http://zakhor/ns/codeLocation")
}

pub fn observation_content_iri() -> &'static Iri {
    iri!("http://zakhor/ns/observationContent")
}

pub fn observation_created_at_iri() -> &'static Iri {
    iri!("http://zakhor/ns/observationCreatedAt")
}

pub fn decision_status_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionStatus")
}

pub fn graph_importance_iri() -> &'static Iri {
    iri!("http://zakhor/ns/graphImportance")
}

pub fn provenance_quality_iri() -> &'static Iri {
    iri!("http://zakhor/ns/provenanceQuality")
}

/// Generate SPARQL CONSTRUCT query that registers the ontology in Tracker.
#[allow(dead_code)]
pub fn ontology_construct_query() -> String {
    let construct = "?s rdf:type ?o .\n\
         ?s rdfs:label ?l .\n\
         ?s rdfs:subClassOf ?sc .\n\
         ?p rdf:type rdf:Property .\n\
         ?p rdfs:domain ?d .\n\
         ?p rdfs:range ?r ."
        .to_string();
    let where_clause = format!(
        "VALUES (?s ?o ?l ?sc) {{\n\
         ({entity} rdf:type rdfs:Class \"Entity\"@en rdfs:Resource)\n\
         ({decision} rdf:type rdfs:Class \"Decision\"@en rdfs:Resource)\n\
         ({project} rdf:type rdfs:Class \"Project\"@en rdfs:Resource)\n\
         ({issue} rdf:type rdfs:Class \"Issue\"@en rdfs:Resource)\n\
         ({constraint} rdf:type rdfs:Class \"Constraint\"@en rdfs:Resource)\n\
         ({observation} rdf:type rdfs:Class \"Observation\"@en rdfs:Resource)\n\
         }}\n\
         VALUES (?p ?d ?r) {{\n\
         ({hasEnt} rdfs:Resource zakhor:Entity)\n\
         ({hasRel} rdfs:Resource rdfs:Resource)\n\
         ({prov} rdfs:Resource rdfs:Resource)\n\
         ({decCtx} zakhor:Decision xsd:string)\n\
         ({decOut} zakhor:Decision xsd:string)\n\
         ({alt} zakhor:Decision xsd:string)\n\
         ({decRat} zakhor:Decision xsd:string)\n\
         }}",
        entity = entity_iri().as_str(),
        decision = decision_iri().as_str(),
        project = project_iri().as_str(),
        issue = issue_iri().as_str(),
        constraint = constraint_iri().as_str(),
        observation = observation_iri().as_str(),
        hasEnt = has_entity_iri().as_str(),
        hasRel = has_relation_iri().as_str(),
        prov = provenance_graph_iri().as_str(),
        decCtx = decision_context_iri().as_str(),
        decOut = decision_outcome_iri().as_str(),
        alt = decision_alternative_iri().as_str(),
        decRat = decision_rationale_iri().as_str(),
    );
    crate::sparql::ontology_construct(&construct, &where_clause)
}

pub fn schema_tool_call_iri() -> &'static Iri {
    iri!("http://zakhor/ns/ToolCall")
}

/// Generate SPARQL INSERT DATA query that registers the ontology in Tracker.
///
/// Uses the same entity/decision/property IRIs as `ontology_construct_query()`
/// but emits explicit `INSERT DATA { … }` triples instead of a CONSTRUCT pattern.
pub fn ontology_insert_query() -> String {
    let e = entity_iri();
    let d = decision_iri();
    let p = project_iri();
    let i = issue_iri();
    let c = constraint_iri();
    let o = observation_iri();
    let he = has_entity_iri();
    let hr = has_relation_iri();
    let pg = provenance_graph_iri();
    let dc = decision_context_iri();
    let do_ = decision_outcome_iri();
    let alt = decision_alternative_iri();
    let dr = decision_rationale_iri();

    let triples = format!(
        "<{e}> rdf:type rdfs:Class ;\n\
               rdfs:label \"Entity\"@en ;\n\
               rdfs:subClassOf rdfs:Resource .\n\
          <{d}> rdf:type rdfs:Class ;\n\
               rdfs:label \"Decision\"@en ;\n\
               rdfs:subClassOf rdfs:Resource .\n\
          <{p}> rdf:type rdfs:Class ;\n\
               rdfs:label \"Project\"@en ;\n\
               rdfs:subClassOf rdfs:Resource .\n\
          <{i}> rdf:type rdfs:Class ;\n\
               rdfs:label \"Issue\"@en ;\n\
               rdfs:subClassOf rdfs:Resource .\n\
          <{c}> rdf:type rdfs:Class ;\n\
               rdfs:label \"Constraint\"@en ;\n\
               rdfs:subClassOf rdfs:Resource .\n\
          <{o}> rdf:type rdfs:Class ;\n\
               rdfs:label \"Observation\"@en ;\n\
               rdfs:subClassOf rdfs:Resource .\n\
          <{he}> rdf:type rdf:Property ;\n\
                 rdfs:domain rdfs:Resource ;\n\
                 rdfs:range zakhor:Entity .\n\
          <{hr}> rdf:type rdf:Property ;\n\
                 rdfs:domain rdfs:Resource ;\n\
                 rdfs:range rdfs:Resource .\n\
          <{pg}> rdf:type rdf:Property ;\n\
                 rdfs:domain rdfs:Resource ;\n\
                 rdfs:range rdfs:Resource .\n\
          <{dc}> rdf:type rdf:Property ;\n\
                 rdfs:domain zakhor:Decision ;\n\
                 rdfs:range xsd:string .\n\
          <{do_}> rdf:type rdf:Property ;\n\
                 rdfs:domain zakhor:Decision ;\n\
                 rdfs:range xsd:string .\n\
          <{alt}> rdf:type rdf:Property ;\n\
                 rdfs:domain zakhor:Decision ;\n\
                 rdfs:range xsd:string .\n\
          <{dr}> rdf:type rdf:Property ;\n\
                 rdfs:domain zakhor:Decision ;\n\
                 rdfs:range xsd:string .",
        e = e.as_str(),
        d = d.as_str(),
        p = p.as_str(),
        i = i.as_str(),
        c = c.as_str(),
        o = o.as_str(),
        he = he.as_str(),
        hr = hr.as_str(),
        pg = pg.as_str(),
        dc = dc.as_str(),
        do_ = do_.as_str(),
        alt = alt.as_str(),
        dr = dr.as_str(),
    );

    SparqlBuilder::insert_data_raw(&triples)
}

/// Generate Turtle/N3 ontology content for use with Tracker SPARQL store.
///
/// Declares the `zakhor:` namespace as an `nrl:Ontology`, all custom classes
/// (Entity, Decision, Project, Issue, Constraint, Observation) and properties
/// (hasEntity, hasRelation, provenanceGraph, decisionContext, decisionRationale)
/// that Zakhor uses.
pub fn ontology_file_content() -> String {
    let mut buf = String::with_capacity(2048);

    // -- @prefix declarations ----------------------------------------------------
    buf.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
    buf.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
    buf.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n");
    buf.push_str("@prefix nrl: <http://tracker.api.gnome.org/ontology/v3/nrl#> .\n");
    buf.push_str("@prefix zakhor: <http://zakhor/ns/> .\n");
    buf.push('\n');

    // -- Ontology declaration ----------------------------------------------------
    buf.push_str("zakhor: a nrl:Namespace, nrl:Ontology ;\n");
    buf.push_str("    nrl:prefix \"zakhor\" ;\n");
    buf.push_str("    nrl:lastModified \"2026-06-19T00:00:00Z\"^^xsd:dateTime .\n");
    buf.push('\n');

    // -- Class definitions -------------------------------------------------------
    for &(name, label) in &[
        ("Entity", "Entity"),
        ("Decision", "Decision"),
        ("Project", "Project"),
        ("Issue", "Issue"),
        ("Constraint", "Constraint"),
        ("Observation", "Observation"),
    ] {
        buf.push_str(&format!(
            concat!(
                "zakhor:{} a rdfs:Class ;\n",
                "    rdfs:label \"{}\"@en ;\n",
                "    rdfs:subClassOf rdfs:Resource .\n",
            ),
            name, label,
        ));
        buf.push('\n');
    }

    // -- Property definitions ----------------------------------------------------
    for &(name, label, domain, range) in &[
        ("hasEntity", "hasEntity", "rdfs:Resource", "zakhor:Entity"),
        (
            "hasRelation",
            "hasRelation",
            "rdfs:Resource",
            "rdfs:Resource",
        ),
        (
            "provenanceGraph",
            "provenanceGraph",
            "rdfs:Resource",
            "rdfs:Resource",
        ),
        (
            "decisionContext",
            "decisionContext",
            "zakhor:Decision",
            "xsd:string",
        ),
        (
            "decisionOutcome",
            "decisionOutcome",
            "zakhor:Decision",
            "xsd:string",
        ),
        (
            "alternative",
            "alternative",
            "zakhor:Decision",
            "xsd:string",
        ),
        (
            "decisionRationale",
            "decisionRationale",
            "zakhor:Decision",
            "xsd:string",
        ),
    ] {
        buf.push_str(&format!(
            concat!(
                "zakhor:{} a rdf:Property ;\n",
                "    rdfs:label \"{}\"@en ;\n",
                "    rdfs:domain {} ;\n",
                "    rdfs:range {} .\n",
            ),
            name, label, domain, range,
        ));
        buf.push('\n');
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_iri_contains_zakhor() {
        let iri = entity_iri();
        assert!(
            iri.as_str().contains("zakhor"),
            "Entity IRI should contain zakhor"
        );
        assert!(
            iri.as_str().contains("Entity"),
            "Entity IRI should contain Entity"
        );
    }

    #[test]
    fn test_decision_iri_contains_zakhor() {
        let iri = decision_iri();
        assert!(iri.as_str().contains("zakhor"));
        assert!(iri.as_str().ends_with("Decision"));
    }

    #[test]
    fn test_has_entity_iri_contains_zakhor() {
        let iri = has_entity_iri();
        assert!(iri.as_str().contains("zakhor"));
        assert!(iri.as_str().ends_with("hasEntity"));
    }

    #[test]
    fn test_construct_query_well_formed() {
        let q = ontology_construct_query();
        assert!(q.starts_with("PREFIX"), "should start with PREFIX");
        assert!(q.contains("CONSTRUCT {"), "should contain CONSTRUCT");
        assert!(q.contains("WHERE {"), "should contain WHERE");
        let opens = q.matches('{').count();
        let closes = q.matches('}').count();
        assert_eq!(
            opens, closes,
            "braces should be balanced: {} opens vs {} closes",
            opens, closes
        );
    }

    #[test]
    fn test_insert_query_well_formed() {
        let q = ontology_insert_query();
        assert!(q.starts_with("PREFIX"), "should start with PREFIX");
        assert!(q.contains("INSERT DATA {"), "should contain INSERT DATA");
        assert!(!q.contains("CONSTRUCT"), "should NOT contain CONSTRUCT");
        // Check all six class IRIs appear (they appear 3× each for type + label + subClassOf)
        assert!(
            q.contains("<http://zakhor/ns/Entity>"),
            "should reference Entity IRI"
        );
        assert!(
            q.contains("<http://zakhor/ns/Decision>"),
            "should reference Decision IRI"
        );
        assert!(
            q.contains("<http://zakhor/ns/Project>"),
            "should reference Project IRI"
        );
        assert!(
            q.contains("<http://zakhor/ns/Issue>"),
            "should reference Issue IRI"
        );
        assert!(
            q.contains("<http://zakhor/ns/Constraint>"),
            "should reference Constraint IRI"
        );
        assert!(
            q.contains("<http://zakhor/ns/Observation>"),
            "should reference Observation IRI"
        );
        // Check property IRIs
        assert!(
            q.contains("<http://zakhor/ns/hasEntity>"),
            "should reference hasEntity IRI"
        );
        assert!(
            q.contains("<http://zakhor/ns/hasRelation>"),
            "should reference hasRelation IRI"
        );
        assert!(
            q.contains("<http://zakhor/ns/provenanceGraph>"),
            "should reference provenanceGraph IRI"
        );
        assert!(
            q.contains("<http://zakhor/ns/decisionContext>"),
            "should reference decisionContext IRI"
        );
        assert!(
            q.contains("<http://zakhor/ns/decisionRationale>"),
            "should reference decisionRationale IRI"
        );
        // Balanced braces
        let opens = q.matches('{').count();
        let closes = q.matches('}').count();
        assert_eq!(
            opens, closes,
            "braces should be balanced: {} opens vs {} closes",
            opens, closes
        );
    }

    #[test]
    fn test_all_six_classes_defined() {
        let iris = [
            entity_iri(),
            decision_iri(),
            project_iri(),
            issue_iri(),
            constraint_iri(),
            observation_iri(),
        ];
        for iri in &iris {
            assert!(
                iri.as_str().starts_with(Prefix::ZAKHOR),
                "class IRI should start with zakhor ns: {}",
                iri
            );
        }
    }

    #[test]
    fn test_all_five_properties_defined() {
        let iris = [
            has_entity_iri(),
            has_relation_iri(),
            provenance_graph_iri(),
            decision_context_iri(),
            decision_outcome_iri(),
            decision_alternative_iri(),
            decision_rationale_iri(),
        ];
        for iri in &iris {
            assert!(
                iri.as_str().starts_with(Prefix::ZAKHOR),
                "property IRI should start with zakhor ns: {}",
                iri
            );
        }
    }

    // -- ontology_file_content tests --------------------------------------------

    #[test]
    fn test_ontology_file_contains_prefixes() {
        let ttl = ontology_file_content();
        assert!(ttl.contains("@prefix rdf:"), "missing rdf prefix");
        assert!(ttl.contains("@prefix rdfs:"), "missing rdfs prefix");
        assert!(ttl.contains("@prefix xsd:"), "missing xsd prefix");
        assert!(ttl.contains("@prefix nrl:"), "missing nrl prefix");
        assert!(ttl.contains("@prefix zakhor:"), "missing zakhor prefix");
        assert!(
            ttl.contains("<http://zakhor/ns/>"),
            "zakhor namespace should match Prefix::ZAKHOR"
        );
    }

    #[test]
    fn test_ontology_file_declares_nrl_ontology() {
        let ttl = ontology_file_content();
        assert!(
            ttl.contains("a nrl:Namespace, nrl:Ontology"),
            "should declare zakhor as nrl:Namespace, nrl:Ontology"
        );
        assert!(
            ttl.contains("nrl:prefix \"zakhor\""),
            "should have nrl:prefix property"
        );
        assert!(
            ttl.contains("nrl:lastModified"),
            "should have nrl:lastModified property"
        );
    }

    #[test]
    fn test_ontology_file_has_all_six_classes() {
        let ttl = ontology_file_content();
        for class in &[
            "Entity",
            "Decision",
            "Project",
            "Issue",
            "Constraint",
            "Observation",
        ] {
            let pattern = format!("zakhor:{} a rdfs:Class", class);
            assert!(
                ttl.contains(&pattern),
                "missing class definition for {}",
                class
            );
        }
    }

    #[test]
    fn test_ontology_file_has_all_five_properties() {
        let ttl = ontology_file_content();
        for prop in &[
            "hasEntity",
            "hasRelation",
            "provenanceGraph",
            "decisionContext",
            "decisionRationale",
        ] {
            let pattern = format!("zakhor:{} a rdf:Property", prop);
            assert!(
                ttl.contains(&pattern),
                "missing property definition for {}",
                prop
            );
        }
    }

    #[test]
    fn test_ontology_file_each_triple_terminated() {
        let ttl = ontology_file_content();
        // Every non-empty, non-prefix line should end with '.', ';', or ',' (Turtle syntax).
        for (i, line) in ttl.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("@prefix") {
                continue;
            }
            let last = trimmed.chars().last().expect("line should not be empty");
            assert!(
                last == '.' || last == ';' || last == ',',
                "line {} ends with unexpected char {:?}: {:?}",
                i + 1,
                last,
                trimmed
            );
        }
    }

    #[test]
    fn test_nrl_constant_correct() {
        assert_eq!(NRL, "http://tracker.api.gnome.org/ontology/v3/nrl#");
    }
}
