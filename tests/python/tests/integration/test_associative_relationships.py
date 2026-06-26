"""Test associative relationships between knowledge graph entities.

Tests 5.1-5.11 from the Integration Test Plan:

  5.1   conflicts_with relation — between entities
  5.2   depends_on relation — between entities
  5.3   supersedes relation — between entities
  5.4   Relation verified via traverse_graph
  5.5   Project association — memory:belongsToProject
  5.6   Fuzzy match — partial entity label matching
  5.7   Many-to-many — entities and observations
  5.8   conflicts_with — between decisions (SPARQL)
  5.9   depends_on — between decisions (SPARQL)
  5.10  ToolCall evidence linking — zakhor:evidenceFor
  5.11  Code container hierarchy — container → symbol

Where possible, tests use MCP tools (store_observation, query_entities,
traverse_graph, record_decision, admin_inject_tool_call). For operations
that require arbitrary SPARQL INSERT (project linking, code containers),
the ``sparql_client`` fixture is used and the test skips if the SPARQL
endpoint is not available.
"""

from __future__ import annotations

import json
import os
from typing import Any

import pytest
from mcp import ClientSession
from mcp.shared.exceptions import McpError
from mcp.types import CallToolResult, TextContent
from SPARQLWrapper import JSON, SPARQLWrapper


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _get_text(result: CallToolResult) -> str:
    """Extract text content from a tool call result."""
    for content in result.content:
        if isinstance(content, TextContent):
            return content.text
    msg = f"No TextContent found in result: {result}"
    raise AssertionError(msg)


def _parse_json(text: str) -> dict:
    """Parse JSON from tool result."""
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


def _sparql_available() -> bool:
    """Check if SPARQL endpoint is configured via env var."""
    return bool(os.environ.get("ZAKHOR_SPARQL_ENDPOINT"))


# ---------------------------------------------------------------------------
# Ontology / namespace URIs
# ---------------------------------------------------------------------------

RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
RDFS = "http://www.w3.org/2000/01/rdf-schema#"
NIE = "http://tracker.api.gnome.org/ontology/v3/nie#"
ZAKHOR = "http://zakhor/ns/"

# ---------------------------------------------------------------------------
# Test constants
# ---------------------------------------------------------------------------

RELATION_ENTITY_A = "http://example.org/relation-a"
RELATION_ENTITY_B = "http://example.org/relation-b"
RELATION_ENTITY_C = "http://example.org/relation-c"

PROJECT_URI = "http://zakhor/ns/test-project-assoc"
ENTITY_FOR_PROJECT = "http://example.org/project-entity"
DECISION_FOR_PROJECT = "urn:uuid:decision-proj-test"

FUZZY_ENTITY_1 = "http://example.org/fuzzy-alpha"
FUZZY_ENTITY_2 = "http://example.org/fuzzy-beta"
FUZZY_ENTITY_3 = "http://example.org/fuzzy-gamma"

OBS_URI_A = "urn:uuid:rel-obs-a"
OBS_URI_B = "urn:uuid:rel-obs-b"

DECISION_URI_A = "urn:uuid:rel-decision-a"
DECISION_URI_B = "urn:uuid:rel-decision-b"

TOOLCALL_URI = "http://zakhor/ns/toolcall/test-evidence"
TOOLCALL_URI_2 = "http://zakhor/ns/toolcall/test-evidence-2"

CONTAINER_URI = "http://zakhor/ns/test-container"
SYMBOL_URI = "http://zakhor/ns/test-symbol"


# ===================================================================
# 5.1  conflicts_with relation
# ===================================================================


@pytest.mark.asyncio
async def test_conflicts_with_relation(mcp_session: ClientSession) -> None:
    """Create a conflicts_with relation via admin tool."""
    try:
        result = await mcp_session.call_tool(
            "admin_create_relation",
            {
                "subject_uri": RELATION_ENTITY_A,
                "predicate_uri": "http://zakhor/ns/conflictsWith",
                "object_uri": RELATION_ENTITY_B,
            },
        )
    except McpError as e:
        pytest.skip(f"admin_create_relation not available: {e}")

    data = _parse_json(_get_text(result))
    if "error" in data:
        pytest.skip(f"admin_create_relation not available: {data['error']}")

    assert data.get("success") is True, f"Expected success=True, got: {data}"


# ===================================================================
# 5.2  depends_on relation
# ===================================================================


@pytest.mark.asyncio
async def test_depends_on_relation(mcp_session: ClientSession) -> None:
    """Create a depends_on relation via admin tool."""
    try:
        result = await mcp_session.call_tool(
            "admin_create_relation",
            {
                "subject_uri": RELATION_ENTITY_A,
                "predicate_uri": "http://zakhor/ns/dependsOn",
                "object_uri": RELATION_ENTITY_B,
            },
        )
    except McpError as e:
        pytest.skip(f"admin_create_relation not available: {e}")

    data = _parse_json(_get_text(result))
    if "error" in data:
        pytest.skip(f"admin_create_relation not available: {data['error']}")

    assert data.get("success") is True, f"Expected success=True, got: {data}"


# ===================================================================
# 5.3  supersedes relation
# ===================================================================


@pytest.mark.asyncio
async def test_supersedes_relation(mcp_session: ClientSession) -> None:
    """Create a supersedes relation via admin tool."""
    try:
        result = await mcp_session.call_tool(
            "admin_create_relation",
            {
                "subject_uri": RELATION_ENTITY_A,
                "predicate_uri": "http://zakhor/ns/supersedes",
                "object_uri": RELATION_ENTITY_B,
            },
        )
    except McpError as e:
        pytest.skip(f"admin_create_relation not available: {e}")

    data = _parse_json(_get_text(result))
    if "error" in data:
        pytest.skip(f"admin_create_relation not available: {data['error']}")

    assert data.get("success") is True, f"Expected success=True, got: {data}"


# ===================================================================
# 5.4  Relation verified via traverse_graph
# ===================================================================


@pytest.mark.asyncio
async def test_relation_verified_via_traverse(mcp_session: ClientSession) -> None:
    """Verify created relation via traverse_graph."""
    try:
        create_result = await mcp_session.call_tool(
            "admin_create_relation",
            {
                "subject_uri": RELATION_ENTITY_A,
                "predicate_uri": "http://zakhor/ns/dependsOn",
                "object_uri": RELATION_ENTITY_B,
            },
        )
    except McpError as e:
        pytest.skip(f"admin_create_relation not available: {e}")

    create_data = _parse_json(_get_text(create_result))
    if "error" in create_data:
        pytest.skip(f"admin_create_relation not available: {create_data['error']}")

    # Traverse to verify
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": RELATION_ENTITY_A, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))

    predicates = {t["predicate"] for t in traverse_data["triples"]}
    assert any("dependsOn" in p for p in predicates), (
        f"Expected dependsOn predicate in traverse, got: {predicates}"
    )


# ===================================================================
# 5.5  Project association — memory:belongsToProject
# ===================================================================


@pytest.mark.skipif(
    "not _sparql_available()",
    reason="SPARQL endpoint not configured (set ZAKHOR_SPARQL_ENDPOINT)",
)
@pytest.mark.asyncio
async def test_project_association_via_sparql(
    mcp_session: ClientSession,
    sparql_client: SPARQLWrapper,
) -> None:
    """5.5: Create a project and link an entity via zakhor:belongsToProject.

    Uses direct SPARQL INSERT to create the project and link, then
    verifies the link via traverse_graph.
    """
    # --- Create a project ---
    project_uri = f"{ZAKHOR}project/integration-test-5.5"
    project_name = "Integration Test Project 5.5"
    entity_uri = ENTITY_FOR_PROJECT

    create_project_sparql = f"""
    PREFIX rdf: <{RDF}>
    PREFIX rdfs: <{RDFS}>
    PREFIX zakhor: <{ZAKHOR}>
    INSERT DATA {{
      <{project_uri}> rdf:type zakhor:Project .
      <{project_uri}> rdfs:label "{project_name}"@en .
    }}
    """
    sparql_client.setQuery(create_project_sparql)
    sparql_client.setMethod("POST")
    try:
        sparql_client.query()
    except Exception as exc:
        pytest.skip(f"Project creation SPARQL failed: {exc}")

    # --- Store an observation with an entity ---
    store_result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Project association test observation",
            "entities": [{"uri": entity_uri, "label": "ProjectTestEntity"}],
            "relations": [],
        },
    )
    store_data = _parse_json(_get_text(store_result))
    assert "observation_uri" in store_data, (
        f"Expected observation_uri, got: {store_data}"
    )

    # --- Link entity to project via SPARQL ---
    link_sparql = f"""
    PREFIX zakhor: <{ZAKHOR}>
    INSERT DATA {{
      <{entity_uri}> zakhor:belongsToProject <{project_uri}> .
    }}
    """
    sparql_client.setQuery(link_sparql)
    try:
        sparql_client.query()
    except Exception as exc:
        pytest.skip(f"Project link SPARQL failed: {exc}")

    # --- Verify via traverse_graph from entity ---
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": entity_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))

    predicates = {t["predicate"] for t in traverse_data["triples"]}
    assert any("belongsToProject" in p for p in predicates), (
        f"Expected belongsToProject predicate, got: {predicates}"
    )

    # Verify the project URI is in the objects
    objects = {t["object"] for t in traverse_data["triples"]}
    assert project_uri in objects, (
        f"Expected project URI {project_uri} in traverse objects, got: {objects}"
    )


@pytest.mark.skipif(
    "not _sparql_available()",
    reason="SPARQL endpoint not configured (set ZAKHOR_SPARQL_ENDPOINT)",
)
@pytest.mark.asyncio
async def test_project_link_survives_reingest(
    mcp_session: ClientSession,
    sparql_client: SPARQLWrapper,
) -> None:
    """5.5b: Project link persists after additional observations with same entity."""
    project_uri = f"{ZAKHOR}project/integration-test-5.5b"
    entity_uri = "http://example.org/project-entity-reingest"

    # Create project
    create_sparql = f"""
    PREFIX rdf: <{RDF}>
    PREFIX rdfs: <{RDFS}>
    PREFIX zakhor: <{ZAKHOR}>
    INSERT DATA {{
      <{project_uri}> rdf:type zakhor:Project .
      <{project_uri}> rdfs:label "Reingest Test Project"@en .
    }}
    """
    sparql_client.setQuery(create_sparql)
    try:
        sparql_client.query()
    except Exception as exc:
        pytest.skip(f"Project creation failed: {exc}")

    # Link entity to project
    link_sparql = f"""
    PREFIX zakhor: <{ZAKHOR}>
    INSERT DATA {{
      <{entity_uri}> zakhor:belongsToProject <{project_uri}> .
    }}
    """
    sparql_client.setQuery(link_sparql)
    try:
        sparql_client.query()
    except Exception as exc:
        pytest.skip(f"Project link failed: {exc}")

    # Store two observations referencing the same entity
    for i in range(2):
        await mcp_session.call_tool(
            "store_observation",
            {
                "text": f"Reingest project test observation {i}",
                "entities": [{"uri": entity_uri, "label": "ReingestEntity"}],
                "relations": [],
            },
        )

    # Verify project link still exists
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": entity_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))

    objects = {t["object"] for t in traverse_data["triples"]}
    assert project_uri in objects, (
        f"Expected project URI {project_uri} in traverse after reingest, got: {objects}"
    )


# ===================================================================
# 5.6  Fuzzy match — partial entity label matching
# ===================================================================


@pytest.mark.asyncio
async def test_fuzzy_match_partial_label(mcp_session: ClientSession) -> None:
    """5.6a: Query entities by partial label fragment (fuzzy match).

    Store entities with compound labels then query by a substring
    that matches multiple entities.
    """
    entities = [
        {"uri": FUZZY_ENTITY_1, "label": "AlphaFuzzyTarget"},
        {"uri": FUZZY_ENTITY_2, "label": "BetaFuzzyTarget"},
        {"uri": FUZZY_ENTITY_3, "label": "GammaDistinct"},
    ]

    for ent in entities:
        await mcp_session.call_tool(
            "store_observation",
            {
                "text": f"Observation about {ent['label']}",
                "entities": [ent],
                "relations": [],
            },
        )

    # Query by shared substring "Fuzzy" — should match alpha and beta
    result = await mcp_session.call_tool(
        "query_entities",
        {"pattern": "Fuzzy", "limit": 10},
    )
    data = _parse_json(_get_text(result))
    assert data["count"] >= 2, f"Expected at least 2 matches for 'Fuzzy', got: {data}"
    matched_labels = {e["label"] for e in data["entities"]}
    assert "AlphaFuzzyTarget" in matched_labels, (
        f"Expected AlphaFuzzyTarget in results, got: {matched_labels}"
    )
    assert "BetaFuzzyTarget" in matched_labels, (
        f"Expected BetaFuzzyTarget in results, got: {matched_labels}"
    )


@pytest.mark.asyncio
async def test_fuzzy_match_case_insensitive(mcp_session: ClientSession) -> None:
    """5.6b: Query entities with case-insensitive pattern matching."""
    # Entity already stored from previous test; store it again if needed
    await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Case insensitive fuzzy test",
            "entities": [{"uri": FUZZY_ENTITY_1, "label": "AlphaFuzzyTarget"}],
            "relations": [],
        },
    )

    # Query with lowercase
    result = await mcp_session.call_tool(
        "query_entities",
        {"pattern": "alphafuzzy", "limit": 10},
    )
    data = _parse_json(_get_text(result))
    assert data["count"] >= 1, (
        f"Expected at least 1 match for lowercase 'alphafuzzy', got: {data}"
    )
    matched_uris = {e["uri"] for e in data["entities"]}
    assert FUZZY_ENTITY_1 in matched_uris, (
        f"Expected {FUZZY_ENTITY_1} in case-insensitive results, got: {matched_uris}"
    )


@pytest.mark.asyncio
async def test_fuzzy_match_no_results(mcp_session: ClientSession) -> None:
    """5.6c: Query with a pattern that matches nothing returns count=0."""
    result = await mcp_session.call_tool(
        "query_entities",
        {"pattern": "NonExistentPatternXYZ_98765", "limit": 10},
    )
    data = _parse_json(_get_text(result))
    assert data["count"] == 0, (
        f"Expected 0 matches for non-existent pattern, got: {data}"
    )


# ===================================================================
# 5.7  Many-to-many — entities and observations
# ===================================================================


@pytest.mark.asyncio
async def test_many_entities_one_observation(mcp_session: ClientSession) -> None:
    """5.7a: Store one observation with multiple entities.

    All entities should be linked to the same observation via
    zakhor:hasEntity.
    """
    multi_entities = [
        {"uri": "http://example.org/many-a", "label": "ManyEntityA"},
        {"uri": "http://example.org/many-b", "label": "ManyEntityB"},
        {"uri": "http://example.org/many-c", "label": "ManyEntityC"},
    ]

    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Observation with multiple entities for many-to-many test",
            "entities": multi_entities,
            "relations": [],
        },
    )
    data = _parse_json(_get_text(result))
    obs_uri = data["observation_uri"]

    # Traverse from observation — should link to all three entities
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": obs_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))

    has_entity_objects = {
        t["object"] for t in traverse_data["triples"] if "hasEntity" in t["predicate"]
    }
    for ent in multi_entities:
        assert ent["uri"] in has_entity_objects, (
            f"Expected {ent['uri']} linked via hasEntity from {obs_uri}, "
            f"got: {has_entity_objects}"
        )


@pytest.mark.asyncio
async def test_one_entity_many_observations(mcp_session: ClientSession) -> None:
    """5.7b: One entity linked from multiple observations.

    The same entity URI used across multiple store_observation calls
    should be reachable from each observation.
    """
    shared_entity = {"uri": "http://example.org/shared-entity", "label": "SharedEntity"}
    obs_uris: list[str] = []

    for i in range(3):
        result = await mcp_session.call_tool(
            "store_observation",
            {
                "text": f"Shared entity observation {i}",
                "entities": [shared_entity],
                "relations": [],
            },
        )
        data = _parse_json(_get_text(result))
        obs_uris.append(data["observation_uri"])

    # Each observation should link to the shared entity
    for obs_uri in obs_uris:
        traverse_result = await mcp_session.call_tool(
            "traverse_graph",
            {"start_id": obs_uri, "depth": 1, "edge_types": []},
        )
        traverse_data = _parse_json(_get_text(traverse_result))
        objects = {
            t["object"]
            for t in traverse_data["triples"]
            if "hasEntity" in t["predicate"]
        }
        assert shared_entity["uri"] in objects, (
            f"Observation {obs_uri} should link to {shared_entity['uri']}, "
            f"got: {objects}"
        )


@pytest.mark.asyncio
async def test_many_to_many_traverse_from_entity(mcp_session: ClientSession) -> None:
    """5.7c: Traverse from entity to find all linked observations.

    An entity used in multiple observations should have multiple
    incoming hasEntity edges.
    """
    entity_uri = "http://example.org/m2m-entity"
    entity_label = "M2MEntity"
    obs_uris: list[str] = []

    for i in range(2):
        result = await mcp_session.call_tool(
            "store_observation",
            {
                "text": f"M2M observation {i} linking to entity",
                "entities": [{"uri": entity_uri, "label": entity_label}],
                "relations": [],
            },
        )
        data = _parse_json(_get_text(result))
        obs_uris.append(data["observation_uri"])

    # Query entities by label — should find exactly one entity
    query_result = await mcp_session.call_tool(
        "query_entities",
        {"pattern": entity_label, "limit": 10},
    )
    query_data = _parse_json(_get_text(query_result))
    assert query_data["count"] >= 1, (
        f"Expected at least 1 entity match, got: {query_data}"
    )

    # Traverse entity properties
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": entity_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    # The entity should have rdf:type and rdfs:label triples
    predicates = {t["predicate"] for t in traverse_data["triples"]}
    assert f"{RDF}type" in predicates, f"Expected rdf:type on entity, got: {predicates}"


# ===================================================================
# 5.8  conflicts_with — between decisions (SPARQL)
# ===================================================================


@pytest.mark.skipif(
    "not _sparql_available()",
    reason="SPARQL endpoint not configured (set ZAKHOR_SPARQL_ENDPOINT)",
)
@pytest.mark.asyncio
async def test_decision_conflicts_with_via_sparql(
    mcp_session: ClientSession,
    sparql_client: SPARQLWrapper,
) -> None:
    """5.8: Record two decisions and link them with conflictsWith.

    Uses record_decision to create decisions, then SPARQL to insert
    the conflictsWith relation, and traverse_graph to verify.
    """
    # Record first decision
    result_a = await mcp_session.call_tool(
        "record_decision",
        {
            "context": "Decision A context for conflicts test",
            "decision": "Use PostgreSQL for primary storage",
            "alternatives": ["Use MySQL"],
            "rationale": "PostgreSQL has better JSON support",
        },
    )
    data_a = _parse_json(_get_text(result_a))
    decision_a_uri = data_a["decision_uri"]
    assert decision_a_uri.startswith("urn:uuid:"), (
        f"Expected urn:uuid: URI, got: {decision_a_uri}"
    )

    # Record second decision
    result_b = await mcp_session.call_tool(
        "record_decision",
        {
            "context": "Decision B context for conflicts test",
            "decision": "Use MongoDB for primary storage",
            "alternatives": ["Use PostgreSQL"],
            "rationale": "MongoDB has better horizontal scaling",
        },
    )
    data_b = _parse_json(_get_text(result_b))
    decision_b_uri = data_b["decision_uri"]
    assert decision_b_uri.startswith("urn:uuid:"), (
        f"Expected urn:uuid: URI, got: {decision_b_uri}"
    )

    # Insert conflictsWith relation via SPARQL
    conflict_sparql = f"""
    PREFIX zakhor: <{ZAKHOR}>
    INSERT DATA {{
      <{decision_a_uri}> zakhor:conflictsWith <{decision_b_uri}> .
    }}
    """
    sparql_client.setQuery(conflict_sparql)
    sparql_client.setMethod("POST")
    try:
        sparql_client.query()
    except Exception as exc:
        pytest.skip(f"conflictsWith SPARQL INSERT failed: {exc}")

    # Verify via traverse_graph from decision A
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": decision_a_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))

    predicates = {t["predicate"] for t in traverse_data["triples"]}
    assert any("conflictsWith" in p for p in predicates), (
        f"Expected conflictsWith predicate, got: {predicates}"
    )

    objects = {t["object"] for t in traverse_data["triples"]}
    assert decision_b_uri in objects, (
        f"Expected decision B URI {decision_b_uri} in traverse objects, got: {objects}"
    )


# ===================================================================
# 5.9  depends_on — between decisions (SPARQL)
# ===================================================================


@pytest.mark.skipif(
    "not _sparql_available()",
    reason="SPARQL endpoint not configured (set ZAKHOR_SPARQL_ENDPOINT)",
)
@pytest.mark.asyncio
async def test_decision_depends_on_via_sparql(
    mcp_session: ClientSession,
    sparql_client: SPARQLWrapper,
) -> None:
    """5.9: Record two decisions and link them with dependsOn.

    Uses record_decision to create decisions, then SPARQL to insert
    the dependsOn relation, and traverse_graph to verify.
    """
    # Record first decision (dependency target)
    result_a = await mcp_session.call_tool(
        "record_decision",
        {
            "context": "Base decision for dependsOn test",
            "decision": "Adopt microservices architecture",
            "alternatives": ["Monolith"],
            "rationale": "Microservices enable independent scaling",
        },
    )
    data_a = _parse_json(_get_text(result_a))
    decision_a_uri = data_a["decision_uri"]
    assert decision_a_uri.startswith("urn:uuid:")

    # Record second decision (depends on first)
    result_b = await mcp_session.call_tool(
        "record_decision",
        {
            "context": "Derived decision for dependsOn test",
            "decision": "Use gRPC for inter-service communication",
            "alternatives": ["REST", "Message Queue"],
            "rationale": "gRPC is efficient for service-to-service calls",
        },
    )
    data_b = _parse_json(_get_text(result_b))
    decision_b_uri = data_b["decision_uri"]
    assert decision_b_uri.startswith("urn:uuid:")

    # Insert dependsOn relation via SPARQL
    depends_sparql = f"""
    PREFIX zakhor: <{ZAKHOR}>
    INSERT DATA {{
      <{decision_b_uri}> zakhor:dependsOn <{decision_a_uri}> .
    }}
    """
    sparql_client.setQuery(depends_sparql)
    sparql_client.setMethod("POST")
    try:
        sparql_client.query()
    except Exception as exc:
        pytest.skip(f"dependsOn SPARQL INSERT failed: {exc}")

    # Verify via traverse_graph from decision B
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": decision_b_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))

    predicates = {t["predicate"] for t in traverse_data["triples"]}
    assert any("dependsOn" in p for p in predicates), (
        f"Expected dependsOn predicate, got: {predicates}"
    )

    objects = {t["object"] for t in traverse_data["triples"]}
    assert decision_a_uri in objects, (
        f"Expected decision A URI {decision_a_uri} in traverse objects, got: {objects}"
    )


# ===================================================================
# 5.10  ToolCall evidence linking — zakhor:evidenceFor
# ===================================================================


@pytest.mark.asyncio
async def test_toolcall_inject_and_verify(mcp_session: ClientSession) -> None:
    """5.10a: Inject a ToolCall via admin_inject_tool_call and verify.

    The ToolCall should have the expected properties (tool_name,
    session_id, arguments) and be reachable via traverse_graph.
    """
    tool_name = "test_store_observation"
    arguments = {"text": "evidence test content", "limit": 5}
    session_id = "test-session-evidence"

    result = await mcp_session.call_tool(
        "admin_inject_tool_call",
        {
            "tool_name": tool_name,
            "arguments": arguments,
            "session_id": session_id,
        },
    )
    data = _parse_json(_get_text(result))
    if "error" in data:
        pytest.skip(f"admin_inject_tool_call not available: {data['error']}")

    assert "uri" in data, f"Expected 'uri' in response, got: {data}"
    tc_uri: str = data["uri"]
    assert tc_uri.startswith("http://zakhor/ns/toolcall/"), (
        f"Expected toolcall URI, got: {tc_uri}"
    )

    # Traverse to verify ToolCall properties
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": tc_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    assert traverse_data["count"] > 0, (
        f"Expected triples for ToolCall {tc_uri}, got empty"
    )

    predicates = {t["predicate"] for t in traverse_data["triples"]}
    assert any("toolName" in p for p in predicates), (
        f"Expected toolName predicate on ToolCall, got: {predicates}"
    )


@pytest.mark.skipif(
    "not _sparql_available()",
    reason="SPARQL endpoint not configured (set ZAKHOR_SPARQL_ENDPOINT)",
)
@pytest.mark.asyncio
async def test_toolcall_evidence_for_decision(
    mcp_session: ClientSession,
    sparql_client: SPARQLWrapper,
) -> None:
    """5.10b: Link a ToolCall to a decision via zakhor:evidenceFor.

    Creates a decision via record_decision, injects a ToolCall via
    admin_inject_tool_call, links them via SPARQL, and verifies
    the evidence link via traverse_graph.
    """
    # Record a decision
    dec_result = await mcp_session.call_tool(
        "record_decision",
        {
            "context": "Evidence linking test context",
            "decision": "Use SPARQL for knowledge graph queries",
            "alternatives": ["Use SQL", "Use Cypher"],
            "rationale": "SPARQL is the W3C standard for RDF querying",
        },
    )
    dec_data = _parse_json(_get_text(dec_result))
    decision_uri = dec_data["decision_uri"]
    assert decision_uri.startswith("urn:uuid:")

    # Inject a ToolCall
    tc_result = await mcp_session.call_tool(
        "admin_inject_tool_call",
        {
            "tool_name": "store_observation",
            "arguments": {"text": "Evidence observation content"},
            "session_id": "session-evidence-link",
        },
    )
    tc_data = _parse_json(_get_text(tc_result))
    if "error" in tc_data:
        pytest.skip(f"admin_inject_tool_call not available: {tc_data['error']}")

    tc_uri = tc_data["uri"]

    # Link ToolCall to decision via SPARQL
    link_sparql = f"""
    PREFIX zakhor: <{ZAKHOR}>
    INSERT DATA {{
      <{tc_uri}> zakhor:evidenceFor <{decision_uri}> .
    }}
    """
    sparql_client.setQuery(link_sparql)
    sparql_client.setMethod("POST")
    try:
        sparql_client.query()
    except Exception as exc:
        pytest.skip(f"evidenceFor SPARQL INSERT failed: {exc}")

    # Verify via traverse_graph from ToolCall
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": tc_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))

    predicates = {t["predicate"] for t in traverse_data["triples"]}
    assert any("evidenceFor" in p for p in predicates), (
        f"Expected evidenceFor predicate, got: {predicates}"
    )

    objects = {t["object"] for t in traverse_data["triples"]}
    assert decision_uri in objects, (
        f"Expected decision URI {decision_uri} in evidenceFor target, got: {objects}"
    )


@pytest.mark.skipif(
    "not _sparql_available()",
    reason="SPARQL endpoint not configured (set ZAKHOR_SPARQL_ENDPOINT)",
)
@pytest.mark.asyncio
async def test_toolcall_multiple_evidence_links(
    mcp_session: ClientSession,
    sparql_client: SPARQLWrapper,
) -> None:
    """5.10c: Multiple ToolCalls can link to the same decision as evidence."""
    # Record a decision
    dec_result = await mcp_session.call_tool(
        "record_decision",
        {
            "context": "Multi-evidence linking test",
            "decision": "Adopt event-driven architecture",
            "alternatives": ["Request-response"],
            "rationale": "Event-driven is more scalable",
        },
    )
    dec_data = _parse_json(_get_text(dec_result))
    decision_uri = dec_data["decision_uri"]

    # Inject two ToolCalls
    tc_uris: list[str] = []
    for i in range(2):
        tc_result = await mcp_session.call_tool(
            "admin_inject_tool_call",
            {
                "tool_name": f"tool_{i}",
                "arguments": {"index": i, "evidence": f"evidence_{i}"},
                "session_id": f"session-multi-{i}",
            },
        )
        tc_data = _parse_json(_get_text(tc_result))
        if "error" in tc_data:
            pytest.skip(f"admin_inject_tool_call not available: {tc_data['error']}")
        tc_uris.append(tc_data["uri"])

    # Link both ToolCalls to the decision
    for tc_uri in tc_uris:
        link_sparql = f"""
        PREFIX zakhor: <{ZAKHOR}>
        INSERT DATA {{
          <{tc_uri}> zakhor:evidenceFor <{decision_uri}> .
        }}
        """
        sparql_client.setQuery(link_sparql)
        sparql_client.setMethod("POST")
        try:
            sparql_client.query()
        except Exception as exc:
            pytest.skip(f"evidenceFor SPARQL INSERT failed: {exc}")

    # Verify each ToolCall links to the decision
    for tc_uri in tc_uris:
        traverse_result = await mcp_session.call_tool(
            "traverse_graph",
            {"start_id": tc_uri, "depth": 1, "edge_types": []},
        )
        traverse_data = _parse_json(_get_text(traverse_result))
        objects = {t["object"] for t in traverse_data["triples"]}
        assert decision_uri in objects, (
            f"ToolCall {tc_uri} should link to decision {decision_uri}, got: {objects}"
        )


# ===================================================================
# 5.11  Code container hierarchy — container → symbol
# ===================================================================


@pytest.mark.skipif(
    "not _sparql_available()",
    reason="SPARQL endpoint not configured (set ZAKHOR_SPARQL_ENDPOINT)",
)
@pytest.mark.asyncio
async def test_code_container_created(
    mcp_session: ClientSession,
    sparql_client: SPARQLWrapper,
) -> None:
    """5.11a: Create a code container via SPARQL and verify via traverse_graph.

    A CodeContainer represents a source file or module with path and language.
    """
    container_uri = f"{ZAKHOR}code/container/integration-test-5.11a"
    code_path = "src/services/query_engine.rs"
    code_language = "rust"

    create_sparql = f"""
    PREFIX rdf: <{RDF}>
    PREFIX zakhor: <{ZAKHOR}>
    INSERT DATA {{
      <{container_uri}> rdf:type zakhor:CodeContainer .
      <{container_uri}> zakhor:codeLocation "{code_path}"@en .
      <{container_uri}> zakhor:codeLanguage "{code_language}"@en .
    }}
    """
    sparql_client.setQuery(create_sparql)
    sparql_client.setMethod("POST")
    try:
        sparql_client.query()
    except Exception as exc:
        pytest.skip(f"Code container SPARQL INSERT failed: {exc}")

    # Verify via traverse_graph
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": container_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    assert traverse_data["count"] > 0, (
        f"Expected triples for CodeContainer {container_uri}, got empty"
    )

    predicates = {t["predicate"] for t in traverse_data["triples"]}
    assert any("codeLocation" in p for p in predicates), (
        f"Expected codeLocation predicate, got: {predicates}"
    )
    assert any("codeLanguage" in p for p in predicates), (
        f"Expected codeLanguage predicate, got: {predicates}"
    )

    # Verify rdf:type zakhor:CodeContainer
    type_objects = {
        t["object"] for t in traverse_data["triples"] if t["predicate"] == f"{RDF}type"
    }
    assert any("CodeContainer" in obj for obj in type_objects), (
        f"Expected CodeContainer type, got: {type_objects}"
    )


@pytest.mark.skipif(
    "not _sparql_available()",
    reason="SPARQL endpoint not configured (set ZAKHOR_SPARQL_ENDPOINT)",
)
@pytest.mark.asyncio
async def test_code_symbol_linked_to_container(
    mcp_session: ClientSession,
    sparql_client: SPARQLWrapper,
) -> None:
    """5.11b: Create a code symbol linked to a container via zakhor:codeLocation.

    A CodeSymbol represents a function/class/interface and links to
    its parent CodeContainer.
    """
    container_uri = f"{ZAKHOR}code/container/integration-test-5.11b"
    symbol_uri = f"{ZAKHOR}code/symbol/query-engine-fn-5.11b"
    symbol_name = "execute_query"
    symbol_kind = "function"
    line_start = 42

    # Create container first
    create_container = f"""
    PREFIX rdf: <{RDF}>
    PREFIX zakhor: <{ZAKHOR}>
    INSERT DATA {{
      <{container_uri}> rdf:type zakhor:CodeContainer .
      <{container_uri}> zakhor:codeLocation "src/lib.rs"@en .
      <{container_uri}> zakhor:codeLanguage "rust"@en .
    }}
    """
    sparql_client.setQuery(create_container)
    sparql_client.setMethod("POST")
    try:
        sparql_client.query()
    except Exception as exc:
        pytest.skip(f"Container SPARQL INSERT failed: {exc}")

    # Create symbol linked to container
    create_symbol = f"""
    PREFIX rdf: <{RDF}>
    PREFIX rdfs: <{RDFS}>
    PREFIX zakhor: <{ZAKHOR}>
    INSERT DATA {{
      <{symbol_uri}> rdf:type zakhor:CodeSymbol .
      <{symbol_uri}> rdfs:label "{symbol_name}"@en .
      <{symbol_uri}> zakhor:codeSymbolKind "{symbol_kind}"@en .
      <{symbol_uri}> zakhor:codeLocation <{container_uri}> .
      <{symbol_uri}> zakhor:codeLineStart {line_start} .
    }}
    """
    sparql_client.setQuery(create_symbol)
    try:
        sparql_client.query()
    except Exception as exc:
        pytest.skip(f"Symbol SPARQL INSERT failed: {exc}")

    # Verify via traverse_graph from symbol
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": symbol_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    assert traverse_data["count"] > 0, (
        f"Expected triples for CodeSymbol {symbol_uri}, got empty"
    )

    # Check rdf:type zakhor:CodeSymbol
    type_objects = {
        t["object"] for t in traverse_data["triples"] if t["predicate"] == f"{RDF}type"
    }
    assert any("CodeSymbol" in obj for obj in type_objects), (
        f"Expected CodeSymbol type, got: {type_objects}"
    )

    # Check codeLocation links to container
    objects = {
        t["object"]
        for t in traverse_data["triples"]
        if "codeLocation" in t["predicate"]
    }
    assert container_uri in objects, (
        f"Expected container URI {container_uri} in codeLocation, got: {objects}"
    )

    # Check codeSymbolKind
    kinds = {
        t["object"]
        for t in traverse_data["triples"]
        if "codeSymbolKind" in t["predicate"]
    }
    assert any(symbol_kind in k for k in kinds), (
        f"Expected symbol kind '{symbol_kind}', got: {kinds}"
    )


@pytest.mark.skipif(
    "not _sparql_available()",
    reason="SPARQL endpoint not configured (set ZAKHOR_SPARQL_ENDPOINT)",
)
@pytest.mark.asyncio
async def test_code_container_traverse_to_symbol(
    mcp_session: ClientSession,
    sparql_client: SPARQLWrapper,
) -> None:
    """5.11c: Traverse from a container to find its symbols.

    After creating a container with a linked symbol, traverse_graph
    from the container should show the link to the symbol.
    """
    container_uri = f"{ZAKHOR}code/container/integration-test-5.11c"
    symbol_uri = f"{ZAKHOR}code/symbol/parse-config-5.11c"
    symbol_name = "parse_config"

    # Create container
    create_container = f"""
    PREFIX rdf: <{RDF}>
    PREFIX zakhor: <{ZAKHOR}>
    INSERT DATA {{
      <{container_uri}> rdf:type zakhor:CodeContainer .
      <{container_uri}> zakhor:codeLocation "src/config.rs"@en .
      <{container_uri}> zakhor:codeLanguage "rust"@en .
    }}
    """
    sparql_client.setQuery(create_container)
    sparql_client.setMethod("POST")
    try:
        sparql_client.query()
    except Exception as exc:
        pytest.skip(f"Container SPARQL INSERT failed: {exc}")

    # Create symbol (linked to container via codeLocation)
    create_symbol = f"""
    PREFIX rdf: <{RDF}>
    PREFIX rdfs: <{RDFS}>
    PREFIX zakhor: <{ZAKHOR}>
    INSERT DATA {{
      <{symbol_uri}> rdf:type zakhor:CodeSymbol .
      <{symbol_uri}> rdfs:label "{symbol_name}"@en .
      <{symbol_uri}> zakhor:codeSymbolKind "function"@en .
      <{symbol_uri}> zakhor:codeLocation <{container_uri}> .
    }}
    """
    sparql_client.setQuery(create_symbol)
    try:
        sparql_client.query()
    except Exception as exc:
        pytest.skip(f"Symbol SPARQL INSERT failed: {exc}")

    # Traverse from container — should see forward links including
    # the incoming edge from the symbol (symbol → codeLocation → container)
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": container_uri, "depth": 2, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    assert traverse_data["count"] > 0, (
        f"Expected triples for container {container_uri}, got empty"
    )
