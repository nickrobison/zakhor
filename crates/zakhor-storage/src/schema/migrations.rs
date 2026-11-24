/// Generate Turtle/N3 ontology content for use with Tracker SPARQL store.
///
/// Declares the `zakhor:` namespace as an `nrl:Ontology`, all custom classes
/// (Entity, Decision, Project, Issue, Constraint, Observation) and properties
/// (hasEntity, hasRelation, provenanceGraph, decisionContext, decisionRationale)
/// that Zakhor uses.
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

    // -- Ontology declaration ----------------------------------------------------
    buf.push_str("zakhor: a nrl:Namespace, nrl:Ontology ;\n");
    buf.push_str("    nrl:prefix \"zakhor\" ;\n");
    buf.push_str("    nrl:lastModified \"2026-11-23T00:00:00Z\"^^xsd:dateTime .\n");
    buf.push('\n');

    // Tracker resolves every property IRI against a declared prefix, so the
    // relation properties get their own namespace instead of nesting under
    // `zakhor:` (a two-segment local name is not prefix-matchable).
    buf.push_str("zrel: a nrl:Namespace, nrl:Ontology ;\n");
    buf.push_str("    nrl:prefix \"zrel\" ;\n");
    buf.push_str("    nrl:lastModified \"2026-11-23T00:00:00Z\"^^xsd:dateTime .\n");
    buf.push('\n');

    // -- Class definitions -------------------------------------------------------
    for &(name, label) in &[
        ("Entity", "Entity"),
        ("Decision", "Decision"),
        ("Project", "Project"),
        ("Repository", "Repository"),
        ("Issue", "Issue"),
        ("Constraint", "Constraint"),
        ("Observation", "Observation"),
        ("ToolCall", "ToolCall"),
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
        (
            "decisionStatus",
            "decisionStatus",
            "zakhor:Decision",
            "xsd:string",
        ),
        (
            "conflictsWith",
            "conflictsWith",
            "zakhor:Decision",
            "rdfs:Resource",
        ),
        ("dependsOn", "dependsOn", "zakhor:Decision", "rdfs:Resource"),
        (
            "supersedes",
            "supersedes",
            "zakhor:Decision",
            "rdfs:Resource",
        ),
        (
            "belongsToProject",
            "belongsToProject",
            "rdfs:Resource",
            "zakhor:Project",
        ),
        (
            "belongsToRepository",
            "belongsToRepository",
            "rdfs:Resource",
            "zakhor:Repository",
        ),
        (
            "evidenceFor",
            "evidenceFor",
            "rdfs:Resource",
            "rdfs:Resource",
        ),
        ("toolName", "toolName", "zakhor:ToolCall", "xsd:string"),
        (
            "toolArguments",
            "toolArguments",
            "zakhor:ToolCall",
            "xsd:string",
        ),
        ("sessionId", "sessionId", "zakhor:ToolCall", "xsd:string"),
        ("timestamp", "timestamp", "zakhor:ToolCall", "xsd:integer"),
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

    // -- Relation predicates for extracted relations -----------------------------
    // `extract_and_store` persists a predicted relation as a plain
    // subject/predicate/object triple whose predicate lives in the `relation`
    // namespace (`zakhor_common::vocab::relation_predicate_iri`). Tracker
    // rejects `INSERT DATA` triples whose predicate is not a declared
    // property, so every configured label needs a property here.
    for label in relation_labels {
        buf.push_str(&format!(
            concat!(
                "zrel:{label} a rdf:Property ;\n",
                "    rdfs:label \"{label}\"@en ;\n",
                "    rdfs:domain rdfs:Resource ;\n",
                "    rdfs:range rdfs:Resource .\n",
            ),
            label = label,
        ));
        buf.push('\n');
    }

    buf
}
