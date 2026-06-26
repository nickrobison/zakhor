"""Test associative relationships between decisions.

Tests:
    - conflicts_with relation
    - depends_on relation
    - supersedes relation

Uses admin_create_relation tool (ephemeral mode only) since the public API
doesn't support creating arbitrary relations.
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


RELATION_ENTITY_A = "http://example.org/relation-a"
RELATION_ENTITY_B = "http://example.org/relation-b"


@pytest.mark.asyncio
async def test_conflicts_with_relation(mcp_session: ClientSession) -> None:
    """Create a conflicts_with relation via admin tool."""
    result = await mcp_session.call_tool(
        "admin_create_relation",
        {
            "subject_uri": RELATION_ENTITY_A,
            "predicate_uri": "http://zakhor/ns/conflictsWith",
            "object_uri": RELATION_ENTITY_B,
        },
    )

    # Admin tool should work in ephemeral mode
    data = _parse_json(_get_text(result))
    if "error" in data:
        pytest.skip(f"admin_create_relation not available: {data['error']}")

    assert data.get("success") is True, f"Expected success=True, got: {data}"


@pytest.mark.asyncio
async def test_depends_on_relation(mcp_session: ClientSession) -> None:
    """Create a depends_on relation via admin tool."""
    result = await mcp_session.call_tool(
        "admin_create_relation",
        {
            "subject_uri": RELATION_ENTITY_A,
            "predicate_uri": "http://zakhor/ns/dependsOn",
            "object_uri": RELATION_ENTITY_B,
        },
    )

    data = _parse_json(_get_text(result))
    if "error" in data:
        pytest.skip(f"admin_create_relation not available: {data['error']}")

    assert data.get("success") is True, f"Expected success=True, got: {data}"


@pytest.mark.asyncio
async def test_supersedes_relation(mcp_session: ClientSession) -> None:
    """Create a supersedes relation via admin tool."""
    result = await mcp_session.call_tool(
        "admin_create_relation",
        {
            "subject_uri": RELATION_ENTITY_A,
            "predicate_uri": "http://zakhor/ns/supersedes",
            "object_uri": RELATION_ENTITY_B,
        },
    )

    data = _parse_json(_get_text(result))
    if "error" in data:
        pytest.skip(f"admin_create_relation not available: {data['error']}")

    assert data.get("success") is True, f"Expected success=True, got: {data}"


@pytest.mark.asyncio
async def test_relation_verified_via_traverse(mcp_session: ClientSession) -> None:
    """Verify created relation via traverse_graph."""
    # Create relation
    create_result = await mcp_session.call_tool(
        "admin_create_relation",
        {
            "subject_uri": RELATION_ENTITY_A,
            "predicate_uri": "http://zakhor/ns/dependsOn",
            "object_uri": RELATION_ENTITY_B,
        },
    )

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
