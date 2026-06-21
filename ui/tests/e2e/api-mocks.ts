import type { Page } from "@playwright/test";

export const apiFixtures = {
  health: { status: "ok" },
  adminStatus: {
    rebuild_in_progress: false,
    lexical_docs: 1280,
    semantic_vectors: 960,
    last_rebuild_at_ms: Date.UTC(2026, 5, 20, 5, 0, 0),
    indexes_available: true,
  },
  rebuild: { status: "accepted", message: "Index rebuild accepted" },
  decisions: {
    decisions: [
      {
        id: "decision-1",
        title: "Use React Flow for graph visualization",
        status: "active",
        created: "2024-01-15T00:00:00.000Z",
        modified: "2024-06-01T00:00:00.000Z",
        confidence: 85,
        evidence_count: 2,
        entity_tags: [{ uri: "entity:react-flow", label: "React Flow" }],
      },
      {
        id: "decision-2",
        title: "Keep API REST-only for web UI",
        status: "active",
        created: "2024-02-01T00:00:00.000Z",
        modified: "2024-06-02T00:00:00.000Z",
        confidence: 92,
        evidence_count: 3,
        entity_tags: [],
      },
      {
        id: "decision-3",
        title: "Archive old design",
        status: "archived",
        created: "2023-01-01T00:00:00.000Z",
        modified: "2024-01-01T00:00:00.000Z",
        confidence: 45,
        evidence_count: 1,
        entity_tags: [{ uri: "entity:old-design", label: "Old Design" }],
      },
    ],
    count: 3,
    total: 3,
  },
  decisionDetail: {
    id: "decision-1",
    title: "Use React Flow for graph visualization",
    status: "active",
    created: "2024-01-15T00:00:00.000Z",
    modified: "2024-06-01T00:00:00.000Z",
    confidence: 85,
    summary: "React Flow provides the best balance of features and performance for our graph visualization needs.",
    context: "Need interactive graph visualization for the knowledge base",
    outcome: "Selected React Flow for its customization and TypeScript support",
    rationale: "React Flow provides the best balance of features and performance",
    alternatives: ["D3.js", "Vis.js", "Sigma.js"],
    evidence: [
      { source: "benchmark-1", content: "React Flow scored highest on performance tests" },
      { source: "user-research", content: "Developers prefer React Flow's API" },
    ],
    entities: [
      { uri: "entity:react-flow", label: "React Flow" },
      { uri: "entity:graph-viz", label: "Graph Visualization" },
    ],
    related_decision_ids: ["decision-2"],
    code_references: [{ file_path: "src/components/graph/GraphView.tsx", repo: "zakhor/zakhor" }],
  },
  provenance: {
    chain: [
      { step: "observation-1", label: "Initial research on graph libraries", source: "research-notes.md" },
      { step: "decision-1", label: "Spike implementation comparison", source: "spike-2024-01.md" },
    ],
    count: 2,
  },
  entities: {
    entities: [
      { uri: "entity:react-flow", label: "React Flow", types: ["Technology"], decision_count: 1, observation_count: 1 },
      { uri: "entity:graph-viz", label: "Graph Visualization", types: ["Component"], decision_count: 2, observation_count: 3 },
    ],
    count: 2,
  },
  entityDetail: {
    uri: "entity:react-flow",
    label: "React Flow",
    types: ["Technology"],
    related_decisions: [{ uri: "decision-1", label: "Use React Flow for graph visualization" }],
    related_observations: [{ uri: "observation-1", text: "React Flow was selected for the graph UI" }],
    relationships: [
      { subject_uri: "entity:react-flow", predicate_uri: "zakhor:usedBy", object_uri: "decision-1", label: "used by" },
    ],
    source_locations: [{ uri: "src/components/graph/GraphView.tsx:12", label: "GraphView.tsx" }],
  },
  entityDecisions: {
    decisions: [{ id: "decision-1", title: "Use React Flow for graph visualization", status: "active" }],
    count: 1,
  },
  entityObservations: {
    observations: [{ uri: "observation-1", text: "React Flow was selected for the graph UI" }],
    count: 1,
  },
  observations: {
    observations: [
      {
        id: "observation-2",
        text: "Graph Explorer should use React Flow instead of a heavier visualization library.",
        created_at: "2024-06-02T00:00:00.000Z",
        confidence: 0.9,
        entity_refs: ["entity:react-flow"],
        source: "design-review.md",
        decision_refs: ["decision-1"],
      },
      {
        id: "observation-1",
        text: "React Flow was selected for the graph UI after comparing alternatives.",
        created_at: "2024-06-01T00:00:00.000Z",
        confidence: 0.85,
        entity_refs: ["entity:react-flow"],
        source: "architecture-notes.md",
        decision_refs: ["decision-1"],
      },
    ],
    count: 2,
    total: 2,
  },
  search: {
    results: [
      { id: "decision:react-flow", score: 0.98 },
      { id: "entity:graph-viz", score: 0.91 },
      { id: "observation:tracker-memory", score: 0.87 },
    ],
    count: 3,
    warning: null,
  },
  graph: {
    triples: [
      { subject: "entity:react-flow", predicate: "zakhor:usedBy", object: "decision-1" },
      { subject: "entity:graph-viz", predicate: "zakhor:relatedTo", object: "entity:react-flow" },
    ],
    count: 2,
    warning: null,
  },
  code: {
    repositories: [
      { name: "zakhor/zakhor", url: "https://github.com/zakhor/zakhor", description: "Zakhor knowledge graph server" },
    ],
    files: [{ path: "src/components/graph/GraphView.tsx", repository: "zakhor/zakhor", language: "TypeScript" }],
    symbols: [{ name: "GraphView", kind: "component", file_path: "src/components/graph/GraphView.tsx", line: 12 }],
  },
};

export async function mockApi(page: Page) {
  await page.route("**/api/v1/**", async (route) => {
    const url = new URL(route.request().url());
    const method = route.request().method();
    const path = decodeURIComponent(url.pathname);

    if (method === "POST" && path === "/api/v1/admin/rebuild-indexes") {
      await route.fulfill({ status: 202, contentType: "application/json", body: JSON.stringify(apiFixtures.rebuild) });
      return;
    }

    if (path === "/api/v1/health") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(apiFixtures.health) });
      return;
    }

    if (path === "/api/v1/admin/status") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(apiFixtures.adminStatus) });
      return;
    }

    if (path === "/api/v1/decisions") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(apiFixtures.decisions) });
      return;
    }

    if (path === "/api/v1/decisions/decision-1") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(apiFixtures.decisionDetail) });
      return;
    }

    if (path === "/api/v1/decisions/decision-1/provenance") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(apiFixtures.provenance) });
      return;
    }

    if (path === "/api/v1/entities") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(apiFixtures.entities) });
      return;
    }

    if (path === "/api/v1/entities/entity:react-flow") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(apiFixtures.entityDetail) });
      return;
    }

    if (path === "/api/v1/entities/entity:react-flow/decisions") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(apiFixtures.entityDecisions) });
      return;
    }

    if (path === "/api/v1/entities/entity:react-flow/observations") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(apiFixtures.entityObservations) });
      return;
    }

    if (path === "/api/v1/observations") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(apiFixtures.observations) });
      return;
    }

    if (path === "/api/v1/search") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(apiFixtures.search) });
      return;
    }

    if (path === "/api/v1/graph/traverse") {
      const warning = url.searchParams.get("depth") === "3" ? "Large graph — consider narrowing search." : null;
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ ...apiFixtures.graph, warning }) });
      return;
    }

    if (path === "/api/v1/code") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(apiFixtures.code) });
      return;
    }

    await route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "not mocked" }) });
  });
}
