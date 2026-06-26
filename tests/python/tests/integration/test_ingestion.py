"""Test the 5-stage ingestion pipeline via store_observation.

Stages:
    1. Validate — check input args are well-formed
    2. Resolve — resolve entity labels to canonical URIs
    3. Build — construct SPARQL INSERT DATA + provenance triples
    4. Persist — execute SPARQL update against triplestore
    5. Track — track provenance in-memory + sync to search indexes
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


OBSERVATION_TEXT = "The system stores observations with entities and relations"
ENTITY_URI = "http://example.org/pipeline-entity"
ENTITY_LABEL = "PipelineTestEntity"


@pytest.mark.asyncio
async def test_store_observation_returns_uri_and_count(
    mcp_session: ClientSession,
) -> None:
    """Stage 1-5: store_observation should return observation_uri and triple_count."""
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": OBSERVATION_TEXT,
            "entities": [{"uri": ENTITY_URI, "label": ENTITY_LABEL}],
            "relations": [],
        },
    )
    data = _parse_json(_get_text(result))
    assert "observation_uri" in data, f"Expected observation_uri, got: {data}"
    assert data["observation_uri"].startswith("urn:uuid:"), (
        f"Expected urn:uuid: URI, got: {data['observation_uri']}"
    )
    assert data.get("triple_count", 0) > 0, f"Expected triple_count > 0, got: {data}"


@pytest.mark.asyncio
async def test_validation_rejects_empty_text(mcp_session: ClientSession) -> None:
    """Stage 1: empty text should be rejected."""
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "",
            "entities": [],
            "relations": [],
        },
    )
    data = _parse_json(_get_text(result))
    # The API should return an error for empty text
    assert result.isError or "error" in str(data).lower(), (
        f"Expected error for empty text, got: {data}"
    )


@pytest.mark.asyncio
async def test_entities_linked_to_observation(mcp_session: ClientSession) -> None:
    """Stage 3-4: entities should be linked via zakhor:hasEntity."""
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Entity linking test",
            "entities": [{"uri": ENTITY_URI, "label": ENTITY_LABEL}],
            "relations": [],
        },
    )
    data = _parse_json(_get_text(result))
    obs_uri = data["observation_uri"]

    # Traverse to verify hasEntity link
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": obs_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    predicates = {t["predicate"] for t in traverse_data["triples"]}

    assert any("hasEntity" in p for p in predicates), (
        f"Expected hasEntity relation, got predicates: {predicates}"
    )


@pytest.mark.asyncio
async def test_entity_has_correct_type(mcp_session: ClientSession) -> None:
    """Stage 3-4: entity should have rdf:type zakhor:Entity."""
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Entity type test",
            "entities": [{"uri": ENTITY_URI, "label": ENTITY_LABEL}],
            "relations": [],
        },
    )
    _parse_json(_get_text(result))

    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": ENTITY_URI, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    predicates = {t["predicate"] for t in traverse_data["triples"]}

    assert any(
        "zakhor/ns/Entity" in t["object"] or "zakhor/ns/Entity" in t["subject"]
        for t in traverse_data["triples"]
    ), (
        f"Expected zakhor:Entity type on entity URI, got triples: {traverse_data['triples']}"
    )


@pytest.mark.asyncio
async def test_relations_persisted(mcp_session: ClientSession) -> None:
    """Stage 4: relations should be stored as SPARQL triples."""
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Relations test",
            "entities": [],
            "relations": [
                {
                    "subject_uri": ENTITY_URI,
                    "predicate_uri": "http://zakhor/ns/relatedTo",
                    "object_uri": "http://example.org/other",
                    "label": "related",
                },
            ],
        },
    )
    data = _parse_json(_get_text(result))

    # Query the subject entity to verify relation
    query_result = await mcp_session.call_tool(
        "query_entities",
        {"pattern": "other", "limit": 10},
    )
    query_data = _parse_json(_get_text(query_result))

    # The relation should exist (object entity may or may not be created)
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": ENTITY_URI, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    predicates = {t["predicate"] for t in traverse_data["triples"]}

    assert any("relatedTo" in p for p in predicates), (
        f"Expected relatedTo predicate, got: {predicates}"
    )
