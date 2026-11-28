//! Generate Turtle/N3 ontology content for use with Tracker SPARQL store.

/// Generate Turtle/N3 ontology content for use with Tracker SPARQL store.
///
/// Renders the canonical Zakhor ontology from
/// [`zakhor_common::vocab::schema`] (`CLASSES` / `PROPERTIES`) into the
/// `nrl:Namespace, nrl:Ontology` declarations Tracker expects, plus the
/// relation properties for the extraction pipeline's configured labels.
///
/// `relation_labels` are the extraction pipeline's configured relation labels:
/// each gets a property under the dedicated `zrel:` namespace
/// (`http://zakhor/ns/relation/{label}`) so `extract_and_store` can persist
/// predicted relations as plain subject/predicate/object triples. Tracker
/// rejects `INSERT DATA` whose predicate is not a declared property, and
/// property IRIs must resolve under a declared prefix — hence the dedicated
/// namespace rather than nesting under `zakhor:`. The labels are always a
/// superset of what the extractor can predict (they are what the extractor is
/// prompted with), so a predicted relation can never hit an undeclared
/// property.
pub fn ontology_file_content(relation_labels: &[String]) -> String {
    let mut buf = String::with_capacity(2048);

    // -- @prefix declarations ----------------------------------------------------
    buf.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
    buf.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
    buf.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n");
    buf.push_str("@prefix nrl: <http://tracker.api.gnome.org/ontology/v3/nrl#> .\n");
    buf.push_str("@prefix zakhor: <http://zakhor/ns/> .\n");
    buf.push_str("@prefix zrel: <http://zakhor/ns/relation/> .\n");
    buf.push('\n');

    // -- Ontology declarations -------------------------------------------------
    // `nrl:lastModified` is derived from the canonical schema content hash, so
    // any schema change automatically bumps the timestamp — existing installs
    // can never keep silently stale ontologies.
    let last_modified = zakhor_common::vocab::schema::ontology_last_modified(relation_labels);
    buf.push_str(&format!(
        "zakhor: a nrl:Namespace, nrl:Ontology ;\n    nrl:prefix \"zakhor\" ;\n    nrl:lastModified \"{last_modified}\"^^xsd:dateTime .\n"
    ));
    buf.push('\n');
    buf.push_str(&format!(
        "zrel: a nrl:Namespace, nrl:Ontology ;\n    nrl:prefix \"zrel\" ;\n    nrl:lastModified \"{last_modified}\"^^xsd:dateTime .\n"
    ));
    buf.push('\n');

    // -- Class definitions -------------------------------------------------------
    for class in zakhor_common::vocab::schema::CLASSES {
        buf.push_str(&format!(
            "zakhor:{} a rdfs:Class ;\n    rdfs:label \"{}\"@en ;\n    rdfs:subClassOf rdfs:Resource .\n\n",
            class.name, class.label,
        ));
    }

    // -- Property definitions ----------------------------------------------------
    for prop in zakhor_common::vocab::schema::PROPERTIES {
        buf.push_str(&format!(
            "zakhor:{} a rdf:Property ;\n    rdfs:label \"{}\"@en ;\n    rdfs:domain {} ;\n    rdfs:range {} .\n\n",
            prop.name, prop.label, prop.domain, prop.range,
        ));
    }

    // -- Relation predicates for extracted relations -----------------------------
    // `extract_and_store` persists a predicted relation as a plain
    // subject/predicate/object triple whose predicate lives in the `relation`
    // namespace (`zakhor_common::vocab::relation_predicate_iri`). Tracker
    // rejects `INSERT DATA` triples whose predicate is not a declared property,
    // so every configured label needs a property here.
    for label in relation_labels {
        buf.push_str(&format!(
            "zrel:{label} a rdf:Property ;\n    rdfs:label \"{label}\"@en ;\n    rdfs:domain rdfs:Resource ;\n    rdfs:range rdfs:Resource .\n\n",
        ));
    }

    buf
}
