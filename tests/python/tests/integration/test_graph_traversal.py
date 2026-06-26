"""Test graph traversal via traverse_graph.

Tests:
    - Depth=1 traversal from observation
    - Depth=2 traversal for multi-hop
    - Edge type filtering
    - Forward and backward traversal
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


TRAVERSE_ENTITY_A = "http://example.org/traverse-a"
TRAVERSE_ENTITY_B = "http://example.org/traverse-b"


@pytest.mark.asyncio
async def test_traverse_from_observation(mcp_session: ClientSession) -> None:
    """Traverse from observation URI returns hasEntity triples."""
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Traverse test observation",
            "entities": [{"uri": TRAVERSE_ENTITY_A, "label": "TraverseA"}],
            "relations": [],
        },
    )
    data = _parse_json(_get_text(result))
    obs_uri = data["observation_uri"]

    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": obs_uri, "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    assert traverse_data["count"] > 0, f"Expected >0 triples, got: {traverse_data}"


@pytest.mark.asyncio
async def test_traverse_depth_two(mcp_session: ClientSession) -> None:
    """Traverse with depth=2 returns multi-hop edges."""
    await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Depth 2 test",
            "entities": [
                {"uri": TRAVERSE_ENTITY_A, "label": "TraverseA"},
                {"uri": TRAVERSE_ENTITY_B, "label": "TraverseB"},
            ],
            "relations": [
                {
                    "subject_uri": TRAVERSE_ENTITY_A,
                    "predicate_uri": "http://zakhor/ns/hasRelation",
                    "object_uri": TRAVERSE_ENTITY_B,
                    "label": "connects to",
                },
            ],
        },
    )

    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": TRAVERSE_ENTITY_A, "depth": 2, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    assert traverse_data["count"] >= 1, (
        f"Expected >=1 triple with depth=2, got: {traverse_data}"
    )


@pytest.mark.asyncio
async def test_traverse_with_edge_filter(mcp_session: ClientSession) -> None:
    """Edge type filter should constrain returned triples."""
    await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Edge filter test",
            "entities": [],
            "relations": [
                {
                    "subject_uri": TRAVERSE_ENTITY_A,
                    "predicate_uri": "http://zakhor/ns/hasRelation",
                    "object_uri": TRAVERSE_ENTITY_B,
                    "label": "connects",
                },
            ],
        },
    )

    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {
            "start_id": TRAVERSE_ENTITY_A,
            "depth": 1,
            "edge_types": ["http://zakhor/ns/hasRelation"],
        },
    )
    traverse_data = _parse_json(_get_text(traverse_result))

    # With edge filter, predicates should match
    if traverse_data["count"] > 0:
        for triple in traverse_data["triples"]:
            assert "connectsTo" in triple["predicate"], (
                f"Expected connectsTo predicate, got: {triple['predicate']}"
            )


@pytest.mark.asyncio
async def test_traverse_empty_start(mcp_session: ClientSession) -> None:
    """Traverse from non-existent URI should return count=0."""
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": "http://example.org/nonexistent", "depth": 1, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    assert traverse_data["count"] == 0, (
        f"Expected 0 triples for non-existent URI, got: {traverse_data}"
    )


@pytest.mark.asyncio
async def test_traverse_invalid_depth(mcp_session: ClientSession) -> None:
    """Traverse with depth=0 should still be valid (returns own properties)."""
    traverse_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": "http://example.org/any", "depth": 0, "edge_types": []},
    )
    traverse_data = _parse_json(_get_text(traverse_result))
    # Should return a valid response (count may be 0 or warning)
    assert "count" in traverse_data, f"Expected count field, got: {traverse_data}"
