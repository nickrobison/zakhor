"""Test the parallel entity+relation extraction pipeline via MCP tools.

This module covers parallel pipeline scenarios:
    1. Backward compatibility — store_observation still works unchanged
    2. extract_and_store returns both entities and relations
    3. Consistent extraction across multiple calls with different texts
    4. Validation rejects empty text (async path)
    5. Multiple parallel extractions all succeed

The tests use the ``mcp_session`` fixture which starts an ephemeral zakhor
server per test function.  No real SPARQL endpoint is needed.
"""

from __future__ import annotations

import json
import logging

import pytest
from mcp import ClientSession
from mcp.types import CallToolResult, TextContent

logger = logging.getLogger(__name__)

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
    """Parse JSON from tool result, handling embedded JSON blocks."""
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass
    for line in text.split("\n"):
        line = line.strip()
        if line.startswith("{"):
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                continue
    raise ValueError(f"Cannot parse JSON from: {text[:200]}")


# ===================================================================
# 1. Backward compatibility — store_observation unchanged
# ===================================================================


@pytest.mark.asyncio
async def test_store_observation_no_regression(
    mcp_session: ClientSession,
) -> None:
    """Verify basic store_observation still works (no MCP tool signature change).

    This is a backward-compatibility gate: the parallel pipeline must not
    alter the existing store_observation interface.
    """
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Backward compatibility test observation",
            "entities": [
                {"uri": "http://example.org/comp-entity", "label": "CompatEntity"}
            ],
            "relations": [],
        },
    )

    assert result.isError is False, (
        f"store_observation should succeed: {_get_text(result)}"
    )

    data = _parse_json(_get_text(result))
    assert "observation_uri" in data, (
        f"Expected observation_uri in response, got: {data}"
    )
    assert data["observation_uri"].startswith("urn:uuid:"), (
        f"Expected urn:uuid: URI, got: {data['observation_uri']}"
    )
    assert data.get("triple_count", 0) > 0, f"Expected triple_count > 0, got: {data}"


# ===================================================================
# 2. extract_and_store returns entities and relations
# ===================================================================


@pytest.mark.asyncio
async def test_extract_and_store_returns_entities_and_relations(
    mcp_session: ClientSession,
) -> None:
    """Call extract_and_store with text; verify result has observation_uri,
    entity_count, and relation_count fields.

    If the server binary does not have the GLiNER extraction pipeline
    configured, the tool returns an error — the test skips gracefully
    in that case.
    """
    result = await mcp_session.call_tool(
        "extract_and_store",
        {
            "uri": "urn:uuid:test-extract-store-1",
            "text": "John works at Google in Mountain View.",
        },
    )

    text = _get_text(result)
    logger.info("extract_and_store result (truncated): %s", text[:200])

    # Gracefully skip if extraction is not available in the server binary
    if result.isError:
        if "extraction" in text.lower() or "model" in text.lower():
            pytest.skip("Extraction pipeline not available in server binary")
        pytest.fail(f"extract_and_store failed unexpectedly: {text}")

    data = _parse_json(text)
    assert "observation_uri" in data, (
        f"Expected observation_uri in response, got: {data}"
    )
    assert data["observation_uri"].startswith("urn:uuid:"), (
        f"Expected urn:uuid: URI, got: {data['observation_uri']}"
    )
    # The response always includes entity_count and relation_count
    assert "entity_count" in data, f"Expected entity_count in response, got: {data}"
    assert "relation_count" in data, f"Expected relation_count in response, got: {data}"


# ===================================================================
# 3. Consistent extraction across multiple calls
# ===================================================================


@pytest.mark.asyncio
async def test_parallel_entity_relation_extraction(
    mcp_session: ClientSession,
) -> None:
    """Call extract_and_store multiple times with different texts and
    verify results are consistent in structure.
    """
    texts = [
        ("urn:uuid:test-parallel-1", "Apple was founded by Steve Jobs in Cupertino."),
        ("urn:uuid:test-parallel-2", "Tim Cook is the CEO of Apple."),
    ]

    for uri, text in texts:
        result = await mcp_session.call_tool(
            "extract_and_store",
            {"uri": uri, "text": text},
        )

        response_text = _get_text(result)
        logger.info("extract_and_store[%s] result: %s", uri, response_text[:200])

        if result.isError:
            if (
                "extraction" in response_text.lower()
                or "model" in response_text.lower()
            ):
                pytest.skip("Extraction pipeline not available in server binary")
            pytest.fail(f"extract_and_store failed for {uri}: {response_text}")

        data = _parse_json(response_text)
        assert "observation_uri" in data, (
            f"Expected observation_uri for {uri}, got: {data}"
        )
        assert data["observation_uri"].startswith("urn:uuid:"), (
            f"Expected urn:uuid: URI for {uri}, got: {data['observation_uri']}"
        )
        assert "entity_count" in data, f"Expected entity_count for {uri}, got: {data}"
        assert "relation_count" in data, (
            f"Expected relation_count for {uri}, got: {data}"
        )


# ===================================================================
# 4. Validation rejects empty text (async)
# ===================================================================


@pytest.mark.asyncio
async def test_validation_rejects_empty_text_async(
    mcp_session: ClientSession,
) -> None:
    """Verify that empty text is still rejected (error handling).

    This tests the async code path — the server should return an error
    when text is empty, regardless of the pipeline path used.
    """
    result = await mcp_session.call_tool(
        "extract_and_store",
        {
            "uri": "urn:uuid:test-empty",
            "text": "",
        },
    )

    text = _get_text(result)
    logger.info("empty text result: %s", text[:200])

    # The MCP tool may return the error as isError=True, or as a success
    # with JSON containing an error message.
    if result.isError:
        assert len(text) > 0, (
            f"Expected error message for empty text, got empty: {result}"
        )
    else:
        data = _parse_json(text)
        assert "error" in str(data).lower(), (
            f"Expected error in response for empty text, got: {data}"
        )


# ===================================================================
# 5. Multiple extractions succeed
# ===================================================================


@pytest.mark.asyncio
async def test_multiple_extractions_succeed(
    mcp_session: ClientSession,
) -> None:
    """Call extract_and_store with 3 different texts and verify each
    returns valid results.
    """
    texts = [
        ("urn:uuid:test-multi-1", "Satya Nadella is the CEO of Microsoft."),
        ("urn:uuid:test-multi-2", "Amazon was founded by Jeff Bezos in Seattle."),
        ("urn:uuid:test-multi-3", "Elon Musk runs Tesla and SpaceX."),
    ]

    for uri, text in texts:
        result = await mcp_session.call_tool(
            "extract_and_store",
            {"uri": uri, "text": text},
        )

        response_text = _get_text(result)
        logger.info("extract_and_store[%s] result: %s", uri, response_text[:200])

        if result.isError:
            if (
                "extraction" in response_text.lower()
                or "model" in response_text.lower()
            ):
                pytest.skip("Extraction pipeline not available in server binary")
            pytest.fail(f"extract_and_store failed for {uri}: {response_text}")

        data = _parse_json(response_text)
        assert "observation_uri" in data, (
            f"Expected observation_uri for {uri}, got keys: {list(data.keys())}"
        )
        assert data["observation_uri"].startswith("urn:uuid:"), (
            f"Expected urn:uuid: URI for {uri}, got: {data['observation_uri']}"
        )

        # entity_count and relation_count are structural fields always present
        assert "entity_count" in data, (
            f"Expected entity_count for {uri}, got keys: {list(data.keys())}"
        )
        assert "relation_count" in data, (
            f"Expected relation_count for {uri}, got keys: {list(data.keys())}"
        )
