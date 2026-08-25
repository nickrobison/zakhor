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
          ({repository} rdf:type rdfs:Class \"Repository\"@en rdfs:Resource)\n\
         ({issue} rdf:type rdfs:Class \"Issue\"@en rdfs:Resource)\n\
         ({constraint} rdf:type rdfs:Class \"Constraint\"@en rdfs:Resource)\n\
          ({observation} rdf:type rdfs:Class \"Observation\"@en rdfs:Resource)\n\
          ({toolCall} rdf:type rdfs:Class \"ToolCall\"@en rdfs:Resource)\n\
          }}\n\
          VALUES (?p ?d ?r) {{\n\
          ({hasEnt} rdfs:Resource zakhor:Entity)\n\
          ({hasRel} rdfs:Resource rdfs:Resource)\n\
          ({prov} rdfs:Resource rdfs:Resource)\n\
          ({decCtx} zakhor:Decision xsd:string)\n\
          ({decOut} zakhor:Decision xsd:string)\n\
          ({alt} zakhor:Decision xsd:string)\n\
          ({decRat} zakhor:Decision xsd:string)\n\
          ({toolName} zakhor:ToolCall xsd:string)\n\
          ({toolArgs} zakhor:ToolCall xsd:string)\n\
          ({sessionId} zakhor:ToolCall xsd:string)\n\
           ({timestamp} zakhor:ToolCall xsd:integer)\n\
           ({btp} rdfs:Resource zakhor:Project)\n\
           ({btr} rdfs:Resource zakhor:Repository)\n\
           }}",
        entity = super::entity_iri().as_str(),
        decision = super::decision_iri().as_str(),
        project = super::project_iri().as_str(),
        repository = super::repository_iri().as_str(),
        issue = super::issue_iri().as_str(),
        constraint = super::constraint_iri().as_str(),
        observation = super::observation_iri().as_str(),
        toolCall = super::schema_tool_call_iri().as_str(),
        hasEnt = super::has_entity_iri().as_str(),
        hasRel = super::has_relation_iri().as_str(),
        prov = super::provenance_graph_iri().as_str(),
        decCtx = super::decision_context_iri().as_str(),
        decOut = super::decision_outcome_iri().as_str(),
        alt = super::decision_alternative_iri().as_str(),
        decRat = super::decision_rationale_iri().as_str(),
        toolName = super::tool_name_iri().as_str(),
        toolArgs = super::tool_arguments_iri().as_str(),
        sessionId = super::session_id_iri().as_str(),
        timestamp = super::timestamp_iri().as_str(),
        btp = super::belongs_to_project_iri().as_str(),
        btr = super::belongs_to_repository_iri().as_str(),
    );
    crate::sparql::ontology_construct(&construct, &where_clause)
}

/// Generate SPARQL INSERT DATA query that registers the ontology in Tracker.
///
/// Uses the same entity/decision/property IRIs as `ontology_construct_query()`
/// but emits explicit `INSERT DATA { … }` triples instead of a CONSTRUCT pattern.
pub fn ontology_insert_query() -> String {
    let e = super::entity_iri();
    let d = super::decision_iri();
    let p = super::project_iri();
    let repo = super::repository_iri();
    let i = super::issue_iri();
    let c = super::constraint_iri();
    let o = super::observation_iri();
    let tc = super::schema_tool_call_iri();
    let he = super::has_entity_iri();
    let hr = super::has_relation_iri();
    let pg = super::provenance_graph_iri();
    let dc = super::decision_context_iri();
    let do_ = super::decision_outcome_iri();
    let alt = super::decision_alternative_iri();
    let dr = super::decision_rationale_iri();
    let tn = super::tool_name_iri();
    let ta = super::tool_arguments_iri();
    let si = super::session_id_iri();
    let ts_iri = super::timestamp_iri();
    let btp = super::belongs_to_project_iri();
    let btr = super::belongs_to_repository_iri();

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
          <{repo}> rdf:type rdfs:Class ;\n\
               rdfs:label \"Repository\"@en ;\n\
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
          <{tc}> rdf:type rdfs:Class ;\n\
               rdfs:label \"ToolCall\"@en ;\n\
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
                   rdfs:range xsd:string .\n\
          <{tn}> rdf:type rdf:Property ;\n\
                  rdfs:domain zakhor:ToolCall ;\n\
                  rdfs:range xsd:string .\n\
          <{ta}> rdf:type rdf:Property ;\n\
                  rdfs:domain zakhor:ToolCall ;\n\
                  rdfs:range xsd:string .\n\
          <{si}> rdf:type rdf:Property ;\n\
                  rdfs:domain zakhor:ToolCall ;\n\
                  rdfs:range xsd:string .\n\
          <{ts_iri}> rdf:type rdf:Property ;\n\
                  rdfs:domain zakhor:ToolCall ;\n\
                  rdfs:range xsd:integer .\n\
          <{btp}> rdf:type rdf:Property ;\n\
                  rdfs:domain rdfs:Resource ;\n\
                  rdfs:range zakhor:Project .\n\
          <{btr}> rdf:type rdf:Property ;\n\
                  rdfs:domain rdfs:Resource ;\n\
                  rdfs:range zakhor:Repository .",
        e = e.as_str(),
        d = d.as_str(),
        p = p.as_str(),
        i = i.as_str(),
        c = c.as_str(),
        o = o.as_str(),
        tc = tc.as_str(),
        he = he.as_str(),
        hr = hr.as_str(),
        pg = pg.as_str(),
        dc = dc.as_str(),
        do_ = do_.as_str(),
        alt = alt.as_str(),
        dr = dr.as_str(),
        tn = tn.as_str(),
        ta = ta.as_str(),
        si = si.as_str(),
        ts_iri = ts_iri.as_str(),
        repo = repo.as_str(),
        btp = btp.as_str(),
        btr = btr.as_str(),
    );

    crate::sparql::SparqlBuilder::insert_data_raw(&triples)
}
