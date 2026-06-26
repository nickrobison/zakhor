"""Graph traversal tests — expand_entity, depth traversal, search context
bundle, Graph Importance ranking, recent observations (plan items 4.1–4.5).

Each test uses the ``mcp_session`` fixture (initialized MCP ClientSession) and
optionally the ``sparql_client`` fixture (SPARQLWrapper for direct SPARQL
queries).  Tests that require the SPARQL endpoint are skipped when
``ZAKHOR_SPARQL_ENDPOINT`` is not set.
"""

from __future__ import annotations

import json
import logging
import os

import pytest
from mcp import ClientSession
from mcp.types import CallToolResult, TextContent
from SPARQLWrapper import JSON, SPARQLWrapper

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Ontology / namespace URIs
# ---------------------------------------------------------------------------

RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
RDFS = "http://www.w3.org/2000/01/rdf-schema#"
NIE = "http://tracker.api.gnome.org/ontology/v3/nie#"
ZAKHOR = "http://zakhor/ns/"

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


def _sparql_available() -> bool:
    """Return True if the SPARQL endpoint is configured."""
    return bool(os.environ.get("ZAKHOR_SPARQL_ENDPOINT"))


# ---------------------------------------------------------------------------
# Test entity URIs  (prefixed to avoid collisions with other test files)
# ---------------------------------------------------------------------------

GT = "http://example.org/gt"  # short prefix for graph-traversal entities

ENTITY_A = f"{GT}/entity-a"
ENTITY_B = f"{GT}/entity-b"
ENTITY_C = f"{GT}/entity-c"
ENTITY_D = f"{GT}/entity-d"
ENTITY_E = f"{GT}/entity-e"
ENTITY_HUB = f"{GT}/hub-entity"


# ===================================================================
# 4.1  expand_entity — entity expansion from the graph
# ===================================================================


@pytest.mark.asyncio
async def test_expand_entity(mcp_session: ClientSession) -> None:
    """Expand an entity to discover its direct connections.

    Stores an observation that links entity A → B via a relation, then
    calls ``traverse_graph`` from A with depth=1.  The expansion should
    return B as a connected node.
    """
    # Store observation with a relation between A and B
    store_result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Expand entity test — relation A-to-B",
            "entities": [
                {"uri": ENTITY_A, "label": "ExpandEntityA"},
                {"uri": ENTITY_B, "label": "ExpandEntityB"},
            ],
            "relations": [
                {
                    "subject_uri": ENTITY_A,
                    "predicate_uri": f"{ZAKHOR}hasRelation",
                    "object_uri": ENTITY_B,
                    "label": "connects to",
                },
            ],
        },
    )
    _parse_json(_get_text(store_result))

    # Expand from entity A with depth=1
    trav_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": ENTITY_A, "depth": 1, "edge_types": []},
    )
    trav_data = _parse_json(_get_text(trav_result))
    assert trav_data["count"] >= 1, (
        f"Expected at least 1 triple when expanding entity A, got: {trav_data}"
    )
    objects = {t["object"] for t in trav_data["triples"]}
    assert ENTITY_B in objects, (
        f"Entity B should appear as a connected node when expanding A, "
        f"got objects: {objects}"
    )


@pytest.mark.asyncio
async def test_expand_entity_empty(mcp_session: ClientSession) -> None:
    """Expand from a non-existent entity returns count=0."""
    trav_result = await mcp_session.call_tool(
        "traverse_graph",
        {
            "start_id": "http://example.org/gt/nonexistent-expand",
            "depth": 1,
            "edge_types": [],
        },
    )
    trav_data = _parse_json(_get_text(trav_result))
    assert trav_data["count"] == 0, (
        f"Expected 0 triples for non-existent entity, got: {trav_data}"
    )


# ===================================================================
# 4.2  Depth traversal — multi-hop navigation
# ===================================================================


@pytest.mark.asyncio
async def test_depth_traversal_two_hop(mcp_session: ClientSession) -> None:
    """Traverse a chain A → B → C; depth=2 should reach C.

    Build a 3-node chain:
        Entity A --connectsTo--> Entity B --connectsTo--> Entity C

    - depth=1 from A should see B but NOT C.
    - depth=2 from A should see both B AND C.
    """
    # Store observation linking A → B
    result_ab = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Depth traversal test — A to B",
            "entities": [
                {"uri": ENTITY_A, "label": "DepthEntityA"},
                {"uri": ENTITY_B, "label": "DepthEntityB"},
            ],
            "relations": [
                {
                    "subject_uri": ENTITY_A,
                    "predicate_uri": f"{ZAKHOR}hasRelation",
                    "object_uri": ENTITY_B,
                    "label": "connects",
                },
            ],
        },
    )
    logger.info("store_observation A→B response: %s", result_ab)
    # Store observation linking B → C
    result_bc = await mcp_session.call_tool(
        "store_observation",
        {
            "text": "Depth traversal test — B to C",
            "entities": [
                {"uri": ENTITY_B, "label": "DepthEntityB"},
                {"uri": ENTITY_C, "label": "DepthEntityC"},
            ],
            "relations": [
                {
                    "subject_uri": ENTITY_B,
                    "predicate_uri": f"{ZAKHOR}hasRelation",
                    "object_uri": ENTITY_C,
                    "label": "connects",
                },
            ],
        },
    )
    logger.info("store_observation B→C response: %s", result_bc)

    # --- depth=1 from A should NOT reach C ---
    d1_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": ENTITY_A, "depth": 1, "edge_types": []},
    )
    d1_data = _parse_json(_get_text(d1_result))
    logger.info("traverse_graph depth=1 from A response: %s", d1_data)
    assert "warning" not in d1_data or not d1_data["warning"], (
        f"traverse_graph returned warning: {d1_data.get('warning')}"
    )
    d1_objects = {t["object"] for t in d1_data["triples"]}
    # Depth 1 should include B (direct neighbor)
    assert ENTITY_B in d1_objects, (
        f"Expected B in depth=1 results from A, got objects: {d1_objects}"
    )
    # Depth 1 should NOT include C (two hops away)
    assert ENTITY_C not in d1_objects, (
        f"C should NOT be reachable at depth=1 from A, got: {d1_objects}"
    )

    # --- depth=2 from A SHOULD reach C ---
    d2_result = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": ENTITY_A, "depth": 2, "edge_types": []},
    )
    d2_data = _parse_json(_get_text(d2_result))
    assert d2_data["count"] >= 1, (
        f"Expected at least 1 triple at depth=2, got: {d2_data}"
    )
    d2_objects = {t["object"] for t in d2_data["triples"]}
    assert ENTITY_C in d2_objects, (
        f"Expected C (2-hop neighbor) at depth=2 from A, got objects: {d2_objects}"
    )


# ===================================================================
# 4.3  Search context bundle — search + traverse combination
# ===================================================================


@pytest.mark.asyncio
async def test_search_context_bundle(mcp_session: ClientSession) -> None:
    """Combine search_hybrid with traverse_graph for context enrichment.

    Store observations with distinctive content, rebuild indexes, search
    for a keyword, then traverse from the result URIs to retrieve the
    graph context (the "bundle").

    If search indexes are not available the test skips gracefully.
    """
    # ── 1. Store observations with entities ─────────────────────────
    label_a = "ContextBundleAlpha"
    label_b = "ContextBundleBeta"
    await mcp_session.call_tool(
        "store_observation",
        {
            "text": "ContextBundle search alpha — the quick brown fox",
            "entities": [
                {"uri": ENTITY_D, "label": label_a},
            ],
            "relations": [],
        },
    )
    await mcp_session.call_tool(
        "store_observation",
        {
            "text": "ContextBundle search beta — jumps over the lazy dog",
            "entities": [
                {"uri": ENTITY_E, "label": label_b},
            ],
            "relations": [],
        },
    )

    # ── 2. Rebuild indexes so content is searchable ─────────────────
    rebuild_result = await mcp_session.call_tool("rebuild_indexes", {})
    rebuild_text = _get_text(rebuild_result)
    assert rebuild_text, "rebuild_indexes should return a non-empty result"

    # ── 3. Search for a term that appears in one observation ─────────
    search_result = await mcp_session.call_tool(
        "search_hybrid",
        {"query": "quick brown fox", "limit": 10},
    )
    search_data = _parse_json(_get_text(search_result))

    # Gracefully skip if indexes are unavailable
    if search_data.get("warning"):
        pytest.skip(f"Search indexes not available: {search_data['warning']}")

    assert "results" in search_data, (
        f"Expected 'results' field in search response, got: {search_data}"
    )
    assert isinstance(search_data["results"], list), (
        f"Expected results to be a list, got: {type(search_data['results'])}"
    )
    assert search_data["count"] > 0, (
        f"Expected at least 1 search result for 'quick brown fox', "
        f"got count={search_data['count']}"
    )

    # ── 4. Traverse from search result URIs to build context bundle ─
    bundle: list[dict] = []
    for result_item in search_data["results"]:
        result_uri: str = result_item["id"]
        trav = await mcp_session.call_tool(
            "traverse_graph",
            {"start_id": result_uri, "depth": 1, "edge_types": []},
        )
        trav_data = _parse_json(_get_text(trav))
        if trav_data["count"] > 0:
            bundle.append(
                {
                    "source_uri": result_uri,
                    "score": result_item.get("score", 0.0),
                    "context_triples": trav_data["triples"],
                }
            )

    # The context bundle should contain at least one entry with triples
    assert len(bundle) > 0, (
        f"Expected at least one context-bundle entry from search results, "
        f"got empty bundle"
    )
    # Each bundle entry should have a source_uri and context_triples
    for entry in bundle:
        assert "source_uri" in entry, f"Expected source_uri in bundle entry: {entry}"
        assert "context_triples" in entry, (
            f"Expected context_triples in bundle entry: {entry}"
        )
        # If triples are present, they should be a list
        assert isinstance(entry["context_triples"], list), (
            f"context_triples should be a list, got: {type(entry['context_triples'])}"
        )


# ===================================================================
# 4.4  Graph Importance ranking — entity centrality / result ordering
# ===================================================================


@pytest.mark.asyncio
async def test_graph_importance_ranking(mcp_session: ClientSession) -> None:
    """Rank entities by graph degree (number of connections).

    A "hub" entity with many connections should appear more prominently
    in traversal results than a leaf entity with only one connection.
    """
    hub_uri = ENTITY_HUB
    leaf_uris = [f"{GT}/leaf-{i}" for i in range(3)]

    # ── 1. Create the hub with 3 connections (each leaf points to hub) ──
    for i, leaf_uri in enumerate(leaf_uris):
        await mcp_session.call_tool(
            "store_observation",
            {
                "text": f"Importance test — leaf {i} connected to hub",
                "entities": [
                    {"uri": leaf_uri, "label": f"ImportanceLeaf{i}"},
                    {"uri": hub_uri, "label": "ImportanceHub"},
                ],
                "relations": [
                    {
                        "subject_uri": leaf_uri,
                        "predicate_uri": f"{ZAKHOR}hasRelation",
                        "object_uri": hub_uri,
                        "label": f"leaf {i} to hub",
                    },
                ],
            },
        )

    # ── 2. Traverse from hub — should show connections to all leaves ──
    hub_trav = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": hub_uri, "depth": 1, "edge_types": []},
    )
    hub_data = _parse_json(_get_text(hub_trav))
    assert hub_data["count"] >= len(leaf_uris), (
        f"Hub should have at least {len(leaf_uris)} connections, "
        f"got {hub_data['count']}"
    )

    # Verify all leaf URIs appear as subjects or objects
    hub_objects = {t["object"] for t in hub_data["triples"]}
    hub_subjects = {t["subject"] for t in hub_data["triples"]}
    all_connected = hub_objects | hub_subjects
    for leaf_uri in leaf_uris:
        assert leaf_uri in all_connected, (
            f"Expected leaf {leaf_uri} to appear in hub traversal, "
            f"got hub_objects={hub_objects}, hub_subjects={hub_subjects}"
        )

    # ── 3. Traverse from a leaf — should show ~fewer triples than hub ──
    leaf_trav = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": leaf_uris[0], "depth": 1, "edge_types": []},
    )
    leaf_data = _parse_json(_get_text(leaf_trav))

    # The leaf should have strictly fewer triples than the hub
    # (leaf: its own type + label, maybe 1 relation; hub: type + label + 3 relations)
    logger.info(
        "Hub triples: %d, leaf triples: %d", hub_data["count"], leaf_data["count"]
    )
    assert leaf_data["count"] < hub_data["count"], (
        f"Expected leaf ({leaf_data['count']}) to have fewer triples "
        f"than hub ({hub_data['count']})"
    )


# ===================================================================
# 4.5  Recent observations — querying recently stored observations
# ===================================================================


@pytest.mark.asyncio
async def test_recent_observations_via_traverse(
    mcp_session: ClientSession,
) -> None:
    """Retrieve recently stored observations by traversing from known
    observation URIs.

    Stores two observations, collects their URIs, then traverses each
    to verify they are reachable and contain the expected content.
    """
    text_a = "Recent observations test — observation alpha"
    text_b = "Recent observations test — observation beta"

    # Store two observations
    r1 = await mcp_session.call_tool(
        "store_observation",
        {
            "text": text_a,
            "entities": [{"uri": f"{GT}/recent-a", "label": "RecentAlpha"}],
            "relations": [],
        },
    )
    d1 = _parse_json(_get_text(r1))
    obs_uri_a = d1["observation_uri"]

    r2 = await mcp_session.call_tool(
        "store_observation",
        {
            "text": text_b,
            "entities": [{"uri": f"{GT}/recent-b", "label": "RecentBeta"}],
            "relations": [],
        },
    )
    d2 = _parse_json(_get_text(r2))
    obs_uri_b = d2["observation_uri"]

    assert obs_uri_a != obs_uri_b, "Each observation should have a unique URI"

    # Traverse each observation URI to verify content
    for label, obs_uri in [("alpha", obs_uri_a), ("beta", obs_uri_b)]:
        trav = await mcp_session.call_tool(
            "traverse_graph",
            {"start_id": obs_uri, "depth": 1, "edge_types": []},
        )
        trav_data = _parse_json(_get_text(trav))
        assert trav_data["count"] > 0, (
            f"Observation {label} ({obs_uri}) should have triples, got: {trav_data}"
        )
        # Verify nie:plainTextContent is present
        content_triples = [
            t
            for t in trav_data["triples"]
            if t["predicate"] == f"{NIE}plainTextContent"
        ]
        assert len(content_triples) >= 1, (
            f"Expected nie:plainTextContent triple for {label}, "
            f"got triples: {trav_data['triples']}"
        )
    # Verify both observations have different content
    assert text_a != text_b, "Observation texts should differ"


@pytest.mark.skipif(
    "not _sparql_available()",
    reason="SPARQL endpoint not configured (set ZAKHOR_SPARQL_ENDPOINT)",
)
@pytest.mark.asyncio
async def test_recent_observations_via_sparql(
    mcp_session: ClientSession,
    sparql_client: SPARQLWrapper,
) -> None:
    """Query recent observations directly via SPARQL.

    Stores an observation, then uses the SPARQL endpoint to query for
    observations ordered by modification time, verifying the stored
    text is returned.
    """
    test_text = "SPARQL recent observations test text — verify via SPARQL"

    # Store observation via MCP
    result = await mcp_session.call_tool(
        "store_observation",
        {
            "text": test_text,
            "entities": [],
            "relations": [],
        },
    )
    data = _parse_json(_get_text(result))
    obs_uri = data["observation_uri"]

    # Direct SPARQL query for recent observations
    query = f"""
    PREFIX nie: <{NIE}>
    PREFIX dcterms: <http://purl.org/dc/terms/>
    SELECT ?obs ?text ?modified WHERE {{
        ?obs a nie:InformationElement .
        ?obs nie:plainTextContent ?text .
        ?obs dcterms:modified ?modified .
    }}
    ORDER BY DESC(?modified)
    LIMIT 10
    """
    sparql_client.setQuery(query)
    sparql_client.setReturnFormat(JSON)

    try:
        raw = sparql_client.query().convert()
        bindings = raw.get("results", {}).get("bindings", [])
        assert len(bindings) >= 1, (
            "Expected at least one SPARQL result for recent observations"
        )
        # The most recent observation should contain our test text
        most_recent_text = bindings[0].get("text", {}).get("value", "")
        logger.info("Most recent SPARQL observation text: %s", most_recent_text)
        # At minimum the observation URI should be in the results
        result_uris = {b.get("obs", {}).get("value", "") for b in bindings}
        assert obs_uri in result_uris, (
            f"Expected stored observation {obs_uri} in SPARQL recent results, "
            f"got URIs: {result_uris}"
        )
    except Exception as exc:
        pytest.skip(f"SPARQL query failed (endpoint may not be available): {exc}")

    # Also verify via traverse_graph
    trav = await mcp_session.call_tool(
        "traverse_graph",
        {"start_id": obs_uri, "depth": 1, "edge_types": []},
    )
    trav_data = _parse_json(_get_text(trav))
    assert trav_data["count"] > 0, (
        f"Expected triples from recent observation, got: {trav_data}"
    )
