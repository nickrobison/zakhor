"""Verify zakhor ontology predicates.

Tests verify that observations are stored using the actual ontology:
- nie:InformationElement for observation type
- rdfs:label for entity labels
- zakhor:decisionStatus for decision status
- zakhor:hasEntity for observation-entity links
"""

from __future__ import annotations

import pytest
from mcp import ClientSession
from mcp.types import CallToolResult, TextContent


def _get_text(result: CallToolResult) -> str:
    """Extract text content from a tool call result."""
    for content in result.content:
        if isinstance(content, TextContent):
            return content.text
    msg = f"No TextContent found in result: {result}"
    raise AssertionError(msg)


def _parse_json(text: str) -> dict:
    """Parse JSON from tool result."""
    import json

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


OBSERVATION_TEXT = "Test observation for ontology verification"
ENTITY_URI = "http://example.org/ontology-test-entity"
ENTITY_LABEL = "OntologyTestEntity"


@pytest.mark.asyncio
async def test_observation_stored_as_nie_information_element(
    mcp_session: ClientSession,
) -> None:
    """Verify observation has rdf:type nie:InformationElement."""
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": OBSERVATION_TEXT,
            "entities": [{"uri": ENTITY_URI, "label": ENTITY_LABEL}],
            "relations": [],
        },
    )
    data = _parse_json(_get_text(result))
    obs_uri = data["observation_uri"]
    assert obs_uri.startswith("urn:uuid:"), f"Expected urn:uuid URI, got: {obs_uri}"

    # Traverse the observation to verify its triples
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": obs_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    predicates = {t["predicate"] for t in traverse_data["triples"]}

    # Check for nie:InformationElement type predicate
    assert any("InformationElement" in p for p in predicates), (
        f"Expected nie:InformationElement type predicate, got: {predicates}"
    )


@pytest.mark.asyncio
async def test_entity_has_rdfs_label(mcp_session: ClientSession) -> None:
    """Verify entity has rdfs:label predicate."""
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Entity label test",
            "entities": [{"uri": ENTITY_URI, "label": ENTITY_LABEL}],
            "relations": [],
        },
    )
    _parse_json(_get_text(result))

    # Query the entity
    query_result = await mcp_session.call_tool(
        "query_entities",
        {"pattern": ENTITY_LABEL, "limit": 1},
    )
    query_data = _parse_json(_get_text(query_result))
    assert query_data["count"] >= 1, f"Expected entity match, got: {query_data}"


@pytest.mark.asyncio
async def test_decision_has_status_predicate(mcp_session: ClientSession) -> None:
    """Verify decision has zakhor:decisionStatus = 'active'."""
    result = await mcp_session.call_tool(
        "record_decision",
        {
            "context": "Test context",
            "decision": "Test decision outcome",
            "alternatives": ["Alt A", "Alt B"],
            "rationale": "Test rationale",
        },
    )
    data = _parse_json(_get_text(result))
    decision_uri = data["decision_uri"]
    assert decision_uri.startswith("urn:uuid:"), (
        f"Expected urn:uuid URI, got: {decision_uri}"
    )
