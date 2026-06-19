use crate::sparql::{Prefix, SparqlBuilder};
use rdf_types::IriBuf;

/// Additional namespace constants (beyond those in Prefix)
pub const NFO: &str = "http://www.semanticdesktop.org/ontologies/2007/03/22/nfo#";
pub const NAO: &str = "http://www.semanticdesktop.org/ontologies/2007/08/15/nao#";
pub const SKOS: &str = "http://www.w3.org/2004/02/skos/core#";

/// All prefix entries for SPARQL queries (adds NFO, NAO, SKOS to basic set from sparql.rs)
pub const EXTRA_PREFIXES: &[(&str, &str)] = &[("nfo", NFO), ("nao", NAO), ("skos", SKOS)];

// IRI constructor functions — simple direct allocation for testability:

pub fn entity_iri() -> IriBuf {
    IriBuf::new(format!("{}Entity", Prefix::ZAKHOR)).unwrap()
}
pub fn decision_iri() -> IriBuf {
    IriBuf::new(format!("{}Decision", Prefix::ZAKHOR)).unwrap()
}
pub fn project_iri() -> IriBuf {
    IriBuf::new(format!("{}Project", Prefix::ZAKHOR)).unwrap()
}
pub fn issue_iri() -> IriBuf {
    IriBuf::new(format!("{}Issue", Prefix::ZAKHOR)).unwrap()
}
pub fn constraint_iri() -> IriBuf {
    IriBuf::new(format!("{}Constraint", Prefix::ZAKHOR)).unwrap()
}
pub fn observation_iri() -> IriBuf {
    IriBuf::new(format!("{}Observation", Prefix::ZAKHOR)).unwrap()
}

pub fn has_entity_iri() -> IriBuf {
    IriBuf::new(format!("{}hasEntity", Prefix::ZAKHOR)).unwrap()
}
pub fn has_relation_iri() -> IriBuf {
    IriBuf::new(format!("{}hasRelation", Prefix::ZAKHOR)).unwrap()
}
pub fn provenance_graph_iri() -> IriBuf {
    IriBuf::new(format!("{}provenanceGraph", Prefix::ZAKHOR)).unwrap()
}
pub fn decision_context_iri() -> IriBuf {
    IriBuf::new(format!("{}decisionContext", Prefix::ZAKHOR)).unwrap()
}
pub fn decision_rationale_iri() -> IriBuf {
    IriBuf::new(format!("{}decisionRationale", Prefix::ZAKHOR)).unwrap()
}

/// Generate SPARQL CONSTRUCT query that registers the ontology in Tracker.
pub fn ontology_construct_query() -> String {
    let construct = format!(
        "?s rdf:type ?o .\n\
         ?s rdfs:label ?l .\n\
         ?s rdfs:subClassOf ?sc .\n\
         ?p rdf:type rdf:Property .\n\
         ?p rdfs:domain ?d .\n\
         ?p rdfs:range ?r ."
    );
    let where_clause = format!(
        "VALUES (?s ?o ?l ?sc) {{\n\
         ({entity} rdf:type owl:Class \"Entity\"@en rdfs:Resource)\n\
         ({decision} rdf:type owl:Class \"Decision\"@en rdfs:Resource)\n\
         ({project} rdf:type owl:Class \"Project\"@en rdfs:Resource)\n\
         ({issue} rdf:type owl:Class \"Issue\"@en rdfs:Resource)\n\
         ({constraint} rdf:type owl:Class \"Constraint\"@en rdfs:Resource)\n\
         ({observation} rdf:type owl:Class \"Observation\"@en rdfs:Resource)\n\
         }}\n\
         VALUES (?p ?d ?r) {{\n\
         ({hasEnt} owl:Thing zakhor:Entity)\n\
         ({hasRel} owl:Thing owl:Thing)\n\
         ({prov} owl:Thing owl:Thing)\n\
         ({decCtx} owl:Thing owl:Thing)\n\
         ({decRat} owl:Thing xsd:string)\n\
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
        decRat = decision_rationale_iri().as_str(),
    );
    SparqlBuilder::construct(&construct, &where_clause)
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
    fn test_extra_prefixes_correct() {
        let nfo = EXTRA_PREFIXES.iter().find(|(k, _)| *k == "nfo");
        assert!(nfo.is_some(), "nfo prefix should exist");
        assert!(
            nfo.unwrap().1.contains("nfo#"),
            "nfo URI should end with nfo#"
        );
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
}
