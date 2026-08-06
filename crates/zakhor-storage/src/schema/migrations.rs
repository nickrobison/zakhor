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
    buf.push_str("    nrl:lastModified \"2026-06-23T00:00:00Z\"^^xsd:dateTime .\n");
    buf.push('\n');

    // -- Class definitions -------------------------------------------------------
    for &(name, label) in &[
        ("Entity", "Entity"),
        ("Decision", "Decision"),
        ("Project", "Project"),
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
            "rdfs:Resource",
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

    buf
}
