import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import React, { useState } from "react";
import { DecisionsPage } from "@/pages/DecisionsPage";
import { DecisionDetailPage } from "@/pages/DecisionDetailPage";
import type { DecisionsResponse, DecisionDetail, ProvenanceResponse } from "@/lib/api/decisions";

const mockDecisionsList: DecisionsResponse = {
  decisions: [
    {
      id: "decision-1",
      title: "Use React Flow for graph visualization",
      status: "active",
      created: "2024-01-15",
      modified: "2024-06-01",
      confidence: 85,
      evidence_count: 2,
      entity_tags: [{ uri: "urn:zakhor:entity:react-flow", label: "React Flow" }],
    },
    {
      id: "decision-2",
      title: "Keep API REST-only for web UI",
      status: "active",
      created: "2024-02-01",
      modified: "2024-06-02",
      confidence: 92,
      evidence_count: 3,
      entity_tags: [],
    },
    {
      id: "decision-3",
      title: "Archive old design",
      status: "archived",
      created: "2023-01-01",
      modified: "2024-01-01",
      confidence: 45,
      evidence_count: 1,
      entity_tags: [{ uri: "urn:zakhor:entity:old-design", label: "Old Design" }],
    },
  ],
  count: 3,
  total: 3,
};

const mockDetailDecision: DecisionDetail = {
  id: "decision-1",
  title: "Use React Flow for graph visualization",
  status: "active",
  created: "2024-01-15",
  modified: "2024-06-01",
  confidence: 85,
  summary: "React Flow provides the best balance of features and performance for our graph visualization needs.",
  context: "Need interactive graph visualization for knowledge base",
  outcome: "Selected React Flow for its customization and TypeScript support",
  rationale: "React Flow provides the best balance of features and performance",
  alternatives: ["D3.js", "Vis.js", "Sigma.js"],
  evidence: [
    { source: "benchmark-1", content: "React Flow scored highest on performance tests" },
    { source: "user-research", content: "Developers prefer React Flow's API" },
  ],
  entities: [
    { uri: "urn:zakhor:entity:react-flow", label: "React Flow" },
    { uri: "urn:zakhor:entity:graph-viz", label: "Graph Visualization" },
  ],
  related_decision_ids: ["decision-2"],
  code_references: [
    { file_path: "src/components/graph/GraphView.tsx", repo: "zakhor/zakhor" },
  ],
};

const mockProvenance: ProvenanceResponse = {
  chain: [
    { step: "observation-1", label: "Initial research on graph libraries", source: "research-notes.md" },
    { step: "decision-1", label: "Spike implementation comparison", source: "spike-2024-01.md" },
  ],
  count: 2,
};

const mocks = vi.hoisted(() => ({
  listDecisions: vi.fn(),
  getDecision: vi.fn(),
  getDecisionProvenance: vi.fn(),
}));

vi.mock("@/lib/api/decisions", () => ({
  listDecisions: mocks.listDecisions,
  getDecision: mocks.getDecision,
  getDecisionProvenance: mocks.getDecisionProvenance,
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => vi.fn(),
  useSearch: () => ({ q: "", status: "active", sort: "modified", limit: 20, offset: 0 }),
  useParams: () => ({ decisionId: "decision-1" }),
  Link: ({ children, to, params }: { children: React.ReactNode; to: string; params?: { decisionId?: string; entityId?: string } }) =>
    params?.decisionId || params?.entityId ? (
      <a href={`${to}/${params.decisionId ?? params.entityId}`} className="block">
        {children}
      </a>
    ) : (
      <a href={to} className="block">
        {children}
      </a>
    ),
}));

vi.mock("@xyflow/react", () => ({
  Background: () => null,
  Controls: () => null,
  MarkerType: { ArrowClosed: "arrowClosed" },
  Position: { Left: "left", Right: "right", Top: "top", Bottom: "bottom" },
  ReactFlow: () => (
    <div data-testid="react-flow">
      Graph visualization
    </div>
  ),
  useEdgesState: (initial: unknown[]) => {
    const [edges] = useState<unknown[]>(initial);
    return [edges, vi.fn(), vi.fn()] as const;
  },
  useNodesState: (initial: unknown[]) => {
    const [nodes] = useState<unknown[]>(initial);
    return [nodes, vi.fn(), vi.fn()] as const;
  },
  useReactFlow: () => ({ fitView: vi.fn() }),
}));

function createQueryClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

function renderDecisionsPage() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <DecisionsPage />
    </QueryClientProvider>,
  );
}

function renderDecisionDetailPage() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <DecisionDetailPage />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("DecisionsPage", () => {
  beforeEach(() => {
    vi.useRealTimers();
    mocks.listDecisions.mockResolvedValue(mockDecisionsList);
  });

  it("renders search controls and filters", () => {
    renderDecisionsPage();

    expect(screen.getByPlaceholderText(/search decisions/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /active/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /archived/i })).toBeInTheDocument();
  });

  it("renders sort dropdown options", () => {
    renderDecisionsPage();

    expect(screen.getByText(/sort by:/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /modified/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /created/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /confidence/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /referenced/i })).toBeInTheDocument();
  });

  it("fetches decisions with correct query parameters", async () => {
    renderDecisionsPage();

    await waitFor(() => {
      expect(mocks.listDecisions).toHaveBeenCalledWith({
        q: "",
        status: "active",
        sort: "modified",
        limit: 20,
        offset: 0,
      });
    });
  });

  it("displays decisions after successful fetch", async () => {
    renderDecisionsPage();

    await waitFor(() => {
      expect(screen.getByText(/react flow for graph visualization/i)).toBeInTheDocument();
    });
  });

  it("shows last-updated date on decision cards", async () => {
    renderDecisionsPage();

    await waitFor(() => {
      expect(screen.getAllByText(/modified:/i).length).toBe(3);
    });
  });

  it("shows decision status badges on cards", async () => {
    renderDecisionsPage();

    await waitFor(() => {
      expect(screen.getAllByText("active").length).toBeGreaterThan(0);
      expect(screen.getByText("archived")).toBeInTheDocument();
    });
  });

  it("displays empty state when no decisions found", async () => {
    mocks.listDecisions.mockResolvedValueOnce({ decisions: [], count: 0, total: 0 });
    renderDecisionsPage();

    await waitFor(() => {
      expect(screen.getByText("No decisions found")).toBeInTheDocument();
    });
  });

  it("shows error message when fetch fails", async () => {
    mocks.listDecisions.mockRejectedValueOnce(new Error("Network error"));
    renderDecisionsPage();

    await waitFor(() => {
      expect(screen.getByText(/failed to load decisions/i)).toBeInTheDocument();
    });
  });

  it("has functional pagination controls", async () => {
    renderDecisionsPage();

    await waitFor(() => screen.getByText(/showing/i));

    expect(screen.getByRole("button", { name: /previous/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /next/i })).toBeInTheDocument();
  });

  it("links cards to decision detail pages", async () => {
    renderDecisionsPage();

    await waitFor(() => {
      const links = screen.getAllByRole("link");
      const decisionLinks = links.filter((link) => link.getAttribute("href")?.includes("/decisions/$decisionId"));
      expect(decisionLinks.length).toBe(3);
    });
  });

  it("makes title the clickable link to decision detail", async () => {
    renderDecisionsPage();

    await waitFor(() => screen.getByText(/showing/i));

    const links = screen.getAllByRole("link");
    const titleLinks = links.filter((link) => link.getAttribute("href") === "/decisions/$decisionId/decision-1");
    expect(titleLinks.length).toBeGreaterThan(0);
  });

  it("displays confidence indicators on decision cards", async () => {
    renderDecisionsPage();

    await waitFor(() => {
      expect(screen.getByText("85%")).toBeInTheDocument();
      expect(screen.getByText("92%")).toBeInTheDocument();
    });
  });

  it("displays evidence count on decision cards", async () => {
    renderDecisionsPage();

    await waitFor(() => {
      expect(screen.getByText("2 evidence")).toBeInTheDocument();
      expect(screen.getByText("3 evidence")).toBeInTheDocument();
      expect(screen.getByText("1 evidence")).toBeInTheDocument();
    });
  });

  it("displays entity tags on decision cards", async () => {
    renderDecisionsPage();

    await waitFor(() => screen.getByText(/showing/i));

    const links = screen.getAllByRole("link");
    const reactFlowLink = links.find((link) => link.getAttribute("href") === "/entities/$entityId/urn:zakhor:entity:react-flow");
    const oldDesignLink = links.find((link) => link.getAttribute("href") === "/entities/$entityId/urn:zakhor:entity:old-design");
    expect(reactFlowLink).toBeInTheDocument();
    expect(oldDesignLink).toBeInTheDocument();
  });
});

describe("DecisionDetailPage", () => {
  beforeEach(() => {
    vi.useRealTimers();
    mocks.getDecision.mockResolvedValue(mockDetailDecision);
    mocks.getDecisionProvenance.mockResolvedValue(mockProvenance);
  });

  it("fetches decision detail and provenance on mount", async () => {
    renderDecisionDetailPage();

    await waitFor(() => {
      expect(mocks.getDecision).toHaveBeenCalledWith("decision-1");
      expect(mocks.getDecisionProvenance).toHaveBeenCalledWith("decision-1");
    });
  });

  it("displays decision title and status", async () => {
    renderDecisionDetailPage();

    await waitFor(() => {
      expect(screen.getByText(/react flow for graph visualization/i)).toBeInTheDocument();
    });
  });

  it("displays dates in card description", async () => {
    renderDecisionDetailPage();

    await waitFor(() => screen.getByText(/modified:/i));

    expect(screen.getByText(/created:/i)).toBeInTheDocument();
  });

  it("displays context, outcome, and rationale sections", async () => {
    renderDecisionDetailPage();

    await waitFor(() => screen.getByText(/react flow for graph visualization/i));

    expect(screen.getByText(/need interactive graph visualization/i)).toBeInTheDocument();
    expect(screen.getByText(/selected react flow/i)).toBeInTheDocument();
    expect(screen.getAllByText(/best balance of features/i).length).toBeGreaterThan(0);
  });

  it("renders evidence tab", async () => {
    renderDecisionDetailPage();

    await waitFor(() => screen.getByRole("tab", { name: /evidence/i }));
  });

  it("renders entities tab", async () => {
    renderDecisionDetailPage();

    await waitFor(() => screen.getByRole("tab", { name: /entities/i }));

    expect(screen.getByRole("tab", { name: /entities/i })).toBeInTheDocument();
  });

  it("renders related decisions tab", async () => {
    renderDecisionDetailPage();

    await waitFor(() => screen.getByRole("tab", { name: /related/i }));

    expect(screen.getByRole("tab", { name: /related/i })).toBeInTheDocument();
  });

  it("links related decisions to real decision detail routes", async () => {
    const user = userEvent.setup();
    renderDecisionDetailPage();

    await waitFor(() => screen.getByRole("tab", { name: /related/i }));

    await user.click(screen.getByRole("tab", { name: /related/i }));

    expect(screen.getByRole("link", { name: "decision-2" })).toHaveAttribute("href", "/decisions/decision-2");
  });

  it("renders code impact tab", async () => {
    renderDecisionDetailPage();

    await waitFor(() => screen.getByRole("tab", { name: /code/i }));

    expect(screen.getByRole("tab", { name: /code/i })).toBeInTheDocument();
  });

  it("renders graph tab", async () => {
    renderDecisionDetailPage();

    await waitFor(() => screen.getByText(/react flow for graph visualization/i));

    expect(screen.getByRole("tab", { name: /graph/i })).toBeInTheDocument();
  });

  it("displays summary section when present", async () => {
    renderDecisionDetailPage();

    await waitFor(() => screen.getByText(/react flow for graph visualization/i));

    expect(screen.getByText(/best balance of features and performance for our graph/i)).toBeInTheDocument();
  });

  it("displays confidence in header", async () => {
    renderDecisionDetailPage();

    await waitFor(() => screen.getByText(/react flow for graph visualization/i));

    const confidenceMatches = screen.getAllByText((_content, element) => {
      return element?.textContent?.includes("85%") ?? false;
    });
    expect(confidenceMatches.length).toBeGreaterThan(0);
  });

  it("displays code references in code tab", async () => {
    renderDecisionDetailPage();

    await waitFor(() => screen.getByText(/react flow for graph visualization/i));

    expect(screen.getByRole("tab", { name: /code/i })).toBeInTheDocument();
  });

  it("renders provenance chain", async () => {
    renderDecisionDetailPage();

    await waitFor(() => {
      expect(screen.getByText(/initial research on graph libraries/i)).toBeInTheDocument();
    });
  });

  it("shows error state when decision fetch fails", async () => {
    mocks.getDecision.mockRejectedValueOnce(new Error("Network error"));
    renderDecisionDetailPage();

    await waitFor(() => {
      expect(screen.getByText(/failed to load decision/i)).toBeInTheDocument();
    });
  });

  it("shows provenance loading state", async () => {
    mocks.getDecisionProvenance.mockImplementation(() => new Promise(() => {}));
    renderDecisionDetailPage();

    await waitFor(() => screen.getByText(/provenance panel/i));

    expect(screen.getByText(/provenance panel/i)).toBeInTheDocument();
  });

  it("collapses and expands provenance panel on header click", async () => {
    renderDecisionDetailPage();

    await waitFor(() => {
      expect(screen.getByText(/initial research on graph libraries/i)).toBeInTheDocument();
    });

    const collapseButton = screen.getByRole("button", { name: /collapse provenance/i });
    await fireEvent.click(collapseButton);

    expect(screen.queryByText(/initial research on graph libraries/i)).not.toBeInTheDocument();

    const expandButton = screen.getByRole("button", { name: /expand provenance/i });
    await fireEvent.click(expandButton);

    expect(screen.getByText(/initial research on graph libraries/i)).toBeInTheDocument();
  });

  it("shows expanded detail when a provenance node is clicked", async () => {
    renderDecisionDetailPage();

    await waitFor(() => {
      expect(screen.getByText(/initial research on graph libraries/i)).toBeInTheDocument();
    });

    const provenanceStep = screen.getByRole("button", { name: /provenance step: observation-1/i });
    await fireEvent.click(provenanceStep);

    expect(screen.getByText(/^Source:/)).toBeInTheDocument();
    expect(screen.getAllByText(/observation-1/).length).toBeGreaterThanOrEqual(2);
  });

  it("renders react-flow graph when related decisions exist", async () => {
    const user = userEvent.setup();
    renderDecisionDetailPage();

    await waitFor(() => screen.getByText(/react flow for graph visualization/i));

    const graphTab = screen.getByRole("tab", { name: /graph/i });
    await user.click(graphTab);

    await waitFor(() => {
      expect(screen.getByTestId("react-flow")).toBeInTheDocument();
    });
  });

  it("shows empty graph message when no related decisions", async () => {
    const user = userEvent.setup();
    mocks.getDecision.mockResolvedValueOnce({
      ...mockDetailDecision,
      related_decision_ids: [],
    });
    renderDecisionDetailPage();

    await waitFor(() => screen.getByText(/react flow for graph visualization/i));

    const graphTab = screen.getByRole("tab", { name: /graph/i });
    await user.click(graphTab);

    await waitFor(() => {
      expect(screen.getByText(/no related decisions to display in graph/i)).toBeInTheDocument();
    });
  });
});
