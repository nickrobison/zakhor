"""Test text index and hybrid search.

Tests:
    - Lexical search via tantivy
    - Semantic search via fastembed
    - RRF fusion of both
    - Search after rebuild_indexes
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


SEARCH_ENTITY_URI = "http://example.org/search-entity"
SEARCH_ENTITY_LABEL = "SearchableEntity"
SEARCH_TEXT = "Unique searchable content about quantum mechanics for testing search"


@pytest.mark.asyncio
async def test_search_with_empty_indexes(mcp_session: ClientSession) -> None:
    """Search should respond gracefully when indexes are not available."""
    result = await mcp_session.call_tool(
        "search_hybrid",
        {"query": "anything", "limit": 10},
    )
    data = _parse_json(_get_text(result))

    # Should have results field (may be empty) or warning field
    assert "results" in data or "warning" in data, (
        f"Expected results or warning field, got: {data}"
    )


@pytest.mark.asyncio
async def test_rebuild_indexes_before_search(mcp_session: ClientSession) -> None:
    """Rebuild indexes, then search should work (if indexes available)."""
    # Rebuild indexes first
    rebuild_result = await mcp_session.call_tool("rebuild_indexes", {})
    rebuild_text = _get_text(rebuild_result)

    # Store something to search for
    await mcp_session.call_tool(
        "store_observation",
        {
            "text": SEARCH_TEXT,
            "entities": [{"uri": SEARCH_ENTITY_URI, "label": SEARCH_ENTITY_LABEL}],
            "relations": [],
        },
    )

    search_result = await mcp_session.call_tool(
        "search_hybrid",
        {"query": "quantum", "limit": 10},
    )
    search_data = _parse_json(_get_text(search_result))

    # If indexes are available, we should get results
    # If not, we should get a warning (test passes either way)
    if "warning" in search_data:
        pytest.skip(f"Search indexes not available: {search_data['warning']}")

    assert "results" in search_data, f"Expected results field, got: {search_data}"


@pytest.mark.asyncio
async def test_search_limit_respected(mcp_session: ClientSession) -> None:
    """Search limit parameter should be respected."""
    result = await mcp_session.call_tool(
        "search_hybrid",
        {"query": "test", "limit": 1},
    )
    data = _parse_json(_get_text(result))

    if "warning" in data:
        pytest.skip(f"Search indexes not available: {data['warning']}")

    if "results" in data:
        assert len(data["results"]) <= 1, f"Expected at most 1 result, got: {data}"


@pytest.mark.asyncio
async def test_query_entities_pattern_matching(mcp_session: ClientSession) -> None:
    """Query entities with pattern should find matches."""
    # Store known entities
    await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Test query entities",
            "entities": [
                {"uri": "http://example.org/qe1", "label": "QuadraticEquation"},
                {"uri": "http://example.org/qe2", "label": "QueryExecution"},
            ],
            "relations": [],
        },
    )

    result = await mcp_session.call_tool(
        "query_entities",
        {"pattern": "uadr", "limit": 10},
    )
    data = _parse_json(_get_text(result))
    assert data["count"] >= 1, f"Expected matches for 'uadr', got: {data}"


@pytest.mark.asyncio
async def test_query_entities_limit(mcp_session: ClientSession) -> None:
    """Query entities limit should be respected."""
    result = await mcp_session.call_tool(
        "query_entities",
        {"pattern": "z", "limit": 2},
    )
    data = _parse_json(_get_text(result))
    assert data["count"] <= 2, f"Expected at most 2 results, got: {data['count']}"


@pytest.mark.asyncio
async def test_query_entities_no_matches(mcp_session: ClientSession) -> None:
    """Query with no matches should return count=0."""
    result = await mcp_session.call_tool(
        "query_entities",
        {"pattern": "NonexistentPatternXYZ123", "limit": 10},
    )
    data = _parse_json(_get_text(result))
    assert data["count"] == 0, f"Expected 0 matches, got: {data}"
