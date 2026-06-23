"""Tests verifying RDF triple structure is stored correctly.

These tests verify that the knowledge graph model (nie:InformationElement,
zakhor:Entity, relations, provenance graphs) is faithfully persisted
by traversing the graph back from stored observations.

Since we don't have direct SPARQL access from the MCP client, we use
traverse_graph and query_entities to introspect stored triples.
"""

from __future__ import annotations

import json

import pytest
from mcp import ClientSession
from mcp.types import CallToolResult, TextContent


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _get_text(result: CallToolResult) -> str:
    for content in result.content:
        if isinstance(content, TextContent):
            return content.text
    msg = f"No TextContent in result: {result}"
    raise AssertionError(msg)


def _parse(text: str) -> dict:
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        for line in text.split("\n"):
            line = line.strip()
            if line.startswith("{"):
                try:
                    return json.loads(line)
                except json.JSONDecodeError:
                    continue
    raise ValueError(f"Cannot parse JSON from: {text[:200]}")


# ---------------------------------------------------------------------------
# Constants  (NIE / RDF ontology prefixes)
# ---------------------------------------------------------------------------

NIE = "http://tracker.api.gnome.org/ontology/v3/nie#"
RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
RDFS = "http://www.w3.org/2000/01/rdf-schema#"
ZAKHOR = "http://zakhor/ns/"


# ===================================================================
# Observation model tests
# ===================================================================


@pytest.mark.asyncio
async def test_observation_is_information_element(mcp_session: ClientSession) -> None:
    """Stored observations should be typed as nie:InformationElement."""
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "RDF model test — information element",
            "entities": [],
            "relations": [],
        },
    )
    data = _parse(_get_text(result))
    obs_uri = data["observation_uri"]

    # Traverse graph from the observation to find type triples
    trav = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": obs_uri, "depth": 1, "edge_types": []},
    )
    trav_data = _parse(_get_text(trav))

    assert trav_data["count"] > 0, "Expected triples for observation"
    triples = trav_data["triples"]

    # Check rdf:type == nie:InformationElement
    type_triples = [
        t
        for t in triples
        if t["predicate"] == f"{RDF}type" and t["object"] == f"{NIE}InformationElement"
    ]
    assert len(type_triples) >= 1, (
        f"Expected rdf:type nie:InformationElement triple, got triples: {triples}"
    )


@pytest.mark.asyncio
async def test_observation_has_text_content(mcp_session: ClientSession) -> None:
    """Observation should have nie:plainTextContent with the stored text."""
    test_text = "Unique text content for RDF model verification"
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": test_text,
            "entities": [],
            "relations": [],
        },
    )
    data = _parse(_get_text(result))
    obs_uri = data["observation_uri"]

    trav = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": obs_uri, "depth": 1, "edge_types": []},
    )
    trav_data = _parse(_get_text(trav))
    triples = trav_data["triples"]

    # Check nie:plainTextContent triple
    content_triples = [t for t in triples if t["predicate"] == f"{NIE}plainTextContent"]
    assert len(content_triples) >= 1, (
        f"Expected nie:plainTextContent triple, got: {triples}"
    )
    # The object should contain the stored text
    assert any(test_text in t["object"] for t in content_triples), (
        f"Expected text '{test_text}' in content triples, got: {content_triples}"
    )


@pytest.mark.asyncio
async def test_entity_is_typed_and_labeled(mcp_session: ClientSession) -> None:
    """Entities should be typed as zakhor:Entity with rdfs:label."""
    entity_uri = "http://example.org/test-entity-42"
    entity_label = "Test Entity 42"

    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Entity model test",
            "entities": [{"uri": entity_uri, "label": entity_label}],
            "relations": [],
        },
    )
    data = _parse(_get_text(result))
    obs_uri = data["observation_uri"]

    # Traverse from the entity to verify its type/label triples
    trav = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": entity_uri, "depth": 1, "edge_types": []},
    )
    trav_data = _parse(_get_text(trav))
    triples = trav_data["triples"]

    # Check rdf:type == zakhor:Entity
    type_triples = [
        t
        for t in triples
        if t["subject"] == entity_uri
        and t["predicate"] == f"{RDF}type"
        and t["object"] == f"{ZAKHOR}Entity"
    ]
    assert len(type_triples) >= 1, (
        f"Expected zakhor:Entity type for entity, got triples: {triples}"
    )

    # Check rdfs:label
    label_triples = [
        t
        for t in triples
        if t["subject"] == entity_uri and t["predicate"] == f"{RDFS}label"
    ]
    assert len(label_triples) >= 1, f"Expected rdfs:label for entity, got: {triples}"
    assert any(entity_label in t["object"] for t in label_triples), (
        f"Expected label '{entity_label}' in label triples, got: {label_triples}"
    )

    # Also verify the observation links to the entity via zakhor:hasEntity
    obs_trav = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": obs_uri, "depth": 1, "edge_types": []},
    )
    obs_data = _parse(_get_text(obs_trav))
    has_entity_triples = [
        t
        for t in obs_data["triples"]
        if t["predicate"] == f"{ZAKHOR}hasEntity" and t["object"] == entity_uri
    ]
    assert len(has_entity_triples) >= 1, (
        f"Expected zakhor:hasEntity linking observation to entity, "
        f"got: {obs_data['triples']}"
    )


@pytest.mark.asyncio
async def test_relation_triple_stored(mcp_session: ClientSession) -> None:
    """Custom relation triples should be persisted as-is."""
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Relation model test",
            "entities": [
                {"uri": "http://example.org/subj", "label": "Subject"},
                {"uri": "http://example.org/obj", "label": "Object"},
            ],
            "relations": [
                {
                    "subject_uri": "http://example.org/subj",
                    "predicate_uri": "http://zakhor/ns/hasRelation",
                    "object_uri": "http://example.org/obj",
                    "label": "related to",
                },
            ],
        },
    )
    data = _parse(_get_text(result))

    # Traverse from subject to find relation
    trav = await mcp_session.call_tool(
        "traverse_graph",
        {
            "start_id": "http://example.org/subj",
            "depth": 1,
            "edge_types": [],
        },
    )
    trav_data = _parse(_get_text(trav))
    triples = trav_data["triples"]

    # Check the relation triple exists
    relation_triples = [
        t
        for t in triples
        if t["subject"] == "http://example.org/subj"
        and t["predicate"] == "http://zakhor/ns/hasRelation"
        and t["object"] == "http://example.org/obj"
    ]
    assert len(relation_triples) >= 1, (
        f"Expected subject/predicate/object relation triple, got: {triples}"
    )


# ===================================================================
# Decision model tests
# ===================================================================


@pytest.mark.asyncio
async def test_decision_model(mcp_session: ClientSession) -> None:
    """Decision entities should be typed correctly.

    Note: decisions are stored as zakhor:Decision (not zakhor:Entity),
    so query_entities (which filters on zakhor:Entity) will NOT find them.
    This test verifies the decision model at the storage level.
    """
    context = "Decision model integration test"
    decision = "Use RDF as the canonical data model"
    alternatives = ["Use JSON", "Use Protobuf"]
    rationale = "RDF enables graph traversal and SPARQL queries"

    result = await mcp_session.call_tool(
        "record_decision",
        {
            "context": context,
            "decision": decision,
            "alternatives": alternatives,
            "rationale": rationale,
        },
    )
    data = _parse(_get_text(result))
    assert "decision_uri" in data, f"Expected decision_uri, got: {data}"
    decision_uri: str = data["decision_uri"]
    assert decision_uri.startswith("urn:uuid:"), (
        f"Expected urn:uuid: URI, got: {decision_uri}"
    )

    # Traverse from the decision URI to verify triples
    trav = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": decision_uri, "depth": 1, "edge_types": []},
    )
    trav_data = _parse(_get_text(trav))
    triples = trav_data["triples"]

    # Check rdf:type zakhor:Decision
    decision_types = [
        t
        for t in triples
        if t["subject"] == decision_uri
        and t["predicate"] == f"{RDF}type"
        and t["object"] == f"{ZAKHOR}Decision"
    ]
    assert len(decision_types) >= 1, (
        f"Expected zakhor:Decision type, got triples: {triples}"
    )

    # Check zakhor:decisionOutcome
    outcome_triples = [
        t
        for t in triples
        if t["subject"] == decision_uri and t["predicate"] == f"{ZAKHOR}decisionOutcome"
    ]
    assert len(outcome_triples) >= 1, f"Expected zakhor:decisionOutcome, got: {triples}"
    assert any(decision in t["object"] for t in outcome_triples), (
        f"Expected decision text in outcome triple, got: {outcome_triples}"
    )

    # Check zakhor:decisionContext
    context_triples = [
        t
        for t in triples
        if t["subject"] == decision_uri and t["predicate"] == f"{ZAKHOR}decisionContext"
    ]
    assert len(context_triples) >= 1, f"Expected zakhor:decisionContext, got: {triples}"

    # Check zakhor:decisionRationale
    rationale_triples = [
        t
        for t in triples
        if t["subject"] == decision_uri
        and t["predicate"] == f"{ZAKHOR}decisionRationale"
    ]
    assert len(rationale_triples) >= 1, (
        f"Expected zakhor:decisionRationale, got: {triples}"
    )

    # Check zakhor:alternative for each alternative
    alt_triples = [
        t
        for t in triples
        if t["subject"] == decision_uri and t["predicate"] == f"{ZAKHOR}alternative"
    ]
    assert len(alt_triples) >= len(alternatives), (
        f"Expected {len(alternatives)} alternative triples, got {len(alt_triples)}: "
        f"{alt_triples}"
    )


# ===================================================================
# Multiple observations isolation
# ===================================================================


@pytest.mark.asyncio
async def test_multiple_observations_isolated(mcp_session: ClientSession) -> None:
    """Each observation should create independent triples.

    Traversing graph should return triples specific to each observation.
    """
    # Store first observation
    r1 = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "First observation — alpha",
            "entities": [{"uri": "http://example.org/alpha", "label": "Alpha"}],
            "relations": [],
        },
    )
    obs1 = _parse(_get_text(r1))["observation_uri"]

    # Store second observation with different entity
    r2 = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Second observation — beta",
            "entities": [{"uri": "http://example.org/beta", "label": "Beta"}],
            "relations": [],
        },
    )
    obs2 = _parse(_get_text(r2))["observation_uri"]

    assert obs1 != obs2, "Each observation should have a unique URI"

    # Traverse graph for each observation
    t1 = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": obs1, "depth": 1, "edge_types": []},
    )
    d1 = _parse(_get_text(t1))

    t2 = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": obs2, "depth": 1, "edge_types": []},
    )
    d2 = _parse(_get_text(t2))

    # Each should have its own content
    texts_1 = {
        t["object"] for t in d1["triples"] if t["predicate"] == f"{NIE}plainTextContent"
    }
    texts_2 = {
        t["object"] for t in d2["triples"] if t["predicate"] == f"{NIE}plainTextContent"
    }
    assert "First observation — alpha" in texts_1, (
        f"Expected alpha text in obs1, got: {texts_1}"
    )
    assert "Second observation — beta" in texts_2, (
        f"Expected beta text in obs2, got: {texts_2}"
    )

    # Alpha entity should NOT appear in beta's triples
    beta_entities = {
        t["object"] for t in d2["triples"] if t["predicate"] == f"{ZAKHOR}hasEntity"
    }
    assert "http://example.org/alpha" not in beta_entities, (
        f"Alpha entity should not appear in beta triples, got: {beta_entities}"
    )
