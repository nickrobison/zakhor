//! SPARQL query builders for the ingestion pipeline.

use zakhor_storage::sparql::{self as storage_sparql, Prefix};

use crate::pipeline::StoreObservationArgs;

// ---------------------------------------------------------------------------
// SPARQL query builder
// ---------------------------------------------------------------------------

fn invalid_iri(what: &str, value: &str) -> crate::errors::IngestionError {
    crate::errors::IngestionError::Validation(
        format!("{what} URI {value:?} is not a valid URI"),
        "build_observation_sparql",
        None,
    )
}

pub use zakhor_storage::sparql::format_iri;

pub use zakhor_storage::sparql::escape_literal;

/// Build the full `INSERT DATA { … }` SPARQL query for an observation.
///
/// `uuid_urn` must be a `urn:uuid:…` string such as `urn:uuid:abc-123`.
pub fn build_observation_sparql(
    args: &StoreObservationArgs,
    uuid_urn: &str,
) -> Result<String, crate::errors::IngestionError> {
    let mut sparql = String::with_capacity(2048);
    sparql.push_str(&storage_sparql::prefix_declarations());
    sparql.push_str("INSERT DATA {\n");

    let uuid_iri = format_iri(uuid_urn).map_err(|e| invalid_iri("observation", &e.0))?;
    let uuid_lit = escape_literal(uuid_urn);
    let text_lit = escape_literal(&args.text);

    sparql.push_str(&format!(
        "  {} rdf:type nie:InformationElement ;\n",
        uuid_iri
    ));
    sparql.push_str(&format!("    nie:identifier {} ;\n", uuid_lit));
    sparql.push_str(&format!("    nie:plainTextContent {} .\n", text_lit));

    for entity in &args.entities {
        let entity_iri =
            format_iri(entity.uri.as_str()).map_err(|e| invalid_iri("entity", &e.0))?;
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
        let subj_iri = format_iri(relation.subject_uri.as_str())
            .map_err(|e| invalid_iri("relation subject", &e.0))?;
        let pred_iri = format_iri(relation.predicate_uri.as_str())
            .map_err(|e| invalid_iri("relation predicate", &e.0))?;
        let obj_iri = format_iri(relation.object_uri.as_str())
            .map_err(|e| invalid_iri("relation object", &e.0))?;
        sparql.push_str(&format!("  {} {} {} .\n", subj_iri, pred_iri, obj_iri,));
    }

    sparql.push_str("}\n");
    Ok(sparql)
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
            entity.uri.as_str().to_string(),
        ));
        triples.push((
            entity.uri.as_str().to_string(),
            format!("{}type", Prefix::RDF),
            format!("{}Entity", Prefix::ZAKHOR),
        ));
        triples.push((
            entity.uri.as_str().to_string(),
            format!("{}label", Prefix::RDFS),
            entity.label.clone(),
        ));
    }

    for relation in &args.relations {
        triples.push((
            relation.subject_uri.as_str().to_string(),
            relation.predicate_uri.as_str().to_string(),
            relation.object_uri.as_str().to_string(),
        ));
    }

    triples
}
