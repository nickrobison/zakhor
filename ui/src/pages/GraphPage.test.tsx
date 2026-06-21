import type { ReactNode } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import { GraphPage } from "@/pages/GraphPage";

const mocks = vi.hoisted(() => ({
  fitView: vi.fn(),
  traverseGraph: vi.fn(),
}));

const mockTriples = [
  { subject: "urn:zakhor:entity:react-flow", predicate: "zakhor:hasRelation", object: "urn:zakhor:entity:react-flow-v12" },
  { subject: "urn:zakhor:entity:react-flow", predicate: "zakhor:dependsOn", object: "urn:zakhor:entity:typescript" },
];

vi.mock("@xyflow/react", async () => {
  const { useState } = await import("react");

  function useNodesState(initialNodes: unknown[]) {
    const [nodes, setNodes] = useState(initialNodes);
    return [nodes, setNodes, vi.fn()] as const;
  }

  function useEdgesState(initialEdges: unknown[]) {
    const [edges, setEdges] = useState(initialEdges);
    return [edges, setEdges, vi.fn()] as const;
  }

  function ReactFlow({ nodes, edges, onNodeClick, children }: { nodes: unknown[]; edges: unknown[]; onNodeClick: (event: unknown, node: { id: string }) => void; children: ReactNode }) {
    return (
      <div data-testid="react-flow" onClick={(event) => onNodeClick(event, { id: "urn:zakhor:entity:react-flow-v12" })}>
        <div data-testid="node-labels">{nodes.map((node) => String((node as { data?: { label?: string } }).data?.label)).join(",")}</div>
        <div data-testid="edge-labels">{edges.map((edge) => String((edge as { label?: string }).label)).join(",")}</div>
        {children}
      </div>
    );
  }

  return {
    Background: () => null,
    Controls: () => null,
    MiniMap: () => null,
    MarkerType: { ArrowClosed: "arrowClosed" },
    Position: { Left: "left", Right: "right", Top: "top", Bottom: "bottom" },
    ReactFlow,
    useEdgesState,
    useNodesState,
    useReactFlow: () => ({ fitView: mocks.fitView }),
  };
});

vi.mock("@/lib/api/graph", () => ({
  traverseGraph: mocks.traverseGraph,
}));

function createQueryClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

function renderGraphPage() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <GraphPage />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("GraphPage", () => {
  it("renders graph controls", () => {
    renderGraphPage();

    expect(screen.getByLabelText(/start node uri/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/depth/i)).toHaveValue("1");
    expect(screen.getByLabelText(/edge predicates/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /traverse/i })).toBeDisabled();
  });

  it("loads graph nodes, edges, and warning at depth 3", async () => {
    mocks.traverseGraph.mockResolvedValue({
      triples: mockTriples,
      count: 2,
      warning: "Large graph — consider narrowing search.",
    });

    renderGraphPage();

    fireEvent.change(screen.getByLabelText(/start node uri/i), { target: { value: "urn:zakhor:entity:react-flow" } });
    fireEvent.change(screen.getByLabelText(/depth/i), { target: { value: "3" } });
    fireEvent.change(screen.getByLabelText(/edge predicates/i), { target: { value: "zakhor:hasRelation" } });
    fireEvent.click(screen.getAllByRole("button", { name: /traverse/i })[0]);

    await waitFor(() => {
      expect(mocks.traverseGraph).toHaveBeenCalledWith("urn:zakhor:entity:react-flow", 3, ["zakhor:hasRelation"]);
      expect(screen.getByText(/large graph/i)).toBeInTheDocument();
      expect(screen.getByText(/react-flow-v12/i)).toBeInTheDocument();
      expect(screen.getAllByText(/hasRelation/i).length).toBeGreaterThan(0);
    });
  });
});
