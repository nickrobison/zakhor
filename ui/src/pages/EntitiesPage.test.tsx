import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { EntitiesPage } from "@/pages/EntitiesPage";
import { EntityDetailPage } from "@/pages/EntityDetailPage";

const mockEntitiesList = {
  entities: [
    { uri: "urn:zakhor:entity:react-flow", label: "React Flow", types: ["Technology"], decision_count: 2, observation_count: 5 },
    { uri: "urn:zakhor:entity:tracker-sparql", label: "Tracker SPARQL", types: ["Component"], decision_count: 1, observation_count: 3 },
  ],
  count: 2,
};

const mockEntityDetail = {
  uri: "urn:zakhor:entity:react-flow",
  label: "React Flow",
  types: ["Technology", "Library"],
  related_decisions: [
    { uri: "urn:zakhor:decision:1", label: "Use React Flow for graph visualization" },
  ],
  related_observations: [
    { uri: "urn:zakhor:observation:1", text: "Selected React Flow for its customization" },
  ],
  relationships: [
    { subject_uri: "urn:zakhor:entity:react-flow", predicate_uri: "urn:zakhor:dependsOn", object_uri: "urn:zakhor:entity:tracker-sparql", label: "dependsOn" },
  ],
  source_locations: [
    { uri: "file:///src/utils/graph.ts", label: "graph.ts" },
  ],
};

const mocks = vi.hoisted(() => ({
  listEntities: vi.fn(),
  getEntity: vi.fn(),
  getEntityDecisions: vi.fn(),
  getEntityObservations: vi.fn(),
}));

vi.mock("@/lib/api/entities", () => ({
  listEntities: mocks.listEntities,
  getEntity: mocks.getEntity,
  getEntityDecisions: mocks.getEntityDecisions,
  getEntityObservations: mocks.getEntityObservations,
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => vi.fn(),
  useSearch: () => ({ q: "", limit: 50 }),
  useParams: () => ({ entityId: "urn:zakhor:entity:react-flow" }),
  Link: ({ children, to, params }: { children: React.ReactNode; to: string; params?: { entityId: string; decisionId?: string } }) =>
    params?.entityId ? (
      <a href={`${to}/${params.entityId}`} className="block">
        {children}
      </a>
    ) : params?.decisionId ? (
      <a href={`${to}/${params.decisionId}`} className="block">
        {children}
      </a>
    ) : (
      <a href={to} className="block">
        {children}
      </a>
    ),
}));

function createQueryClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } });
}

function renderEntitiesPage() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <EntitiesPage />
    </QueryClientProvider>,
  );
}

function renderEntityDetailPage() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <EntityDetailPage />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  vi.resetAllMocks();
});

describe("EntitiesPage", () => {
  beforeEach(() => {
    vi.useRealTimers();
    mocks.listEntities.mockResolvedValue(mockEntitiesList);
  });

  it("renders search input", () => {
    renderEntitiesPage();
    expect(screen.getByPlaceholderText(/search entities by label/i)).toBeInTheDocument();
  });

  it("fetches entities with correct query parameters", async () => {
    renderEntitiesPage();

    await waitFor(() => {
      expect(mocks.listEntities).toHaveBeenCalledWith("", 50);
    });
  });

  it("displays entities after successful fetch", async () => {
    renderEntitiesPage();

    await waitFor(() => {
      expect(screen.getByText(/react flow/i)).toBeInTheDocument();
    });
  });

  it("renders type badges on entity rows", async () => {
    renderEntitiesPage();

    await waitFor(() => screen.getByText(/react flow/i));

    expect(screen.getByText("Technology")).toBeInTheDocument();
    expect(screen.getByText("Component")).toBeInTheDocument();
  });

  it("displays decision and observation counts", async () => {
    renderEntitiesPage();

    await waitFor(() => screen.getByText(/react flow/i));

    expect(screen.getAllByText("2").length).toBeGreaterThan(0);
    expect(screen.getAllByText("1").length).toBeGreaterThan(0);
  });

  it("shows entity total count", async () => {
    renderEntitiesPage();

    await waitFor(() => screen.getByText(/total:/i));

    expect(screen.getByText(/total: 2/i)).toBeInTheDocument();
  });

  it("displays empty state when no entities found", async () => {
    mocks.listEntities.mockResolvedValueOnce({ entities: [], count: 0 });
    renderEntitiesPage();

    await waitFor(() => {
      expect(screen.getByText("No entities found")).toBeInTheDocument();
    });
  });

  it("shows error message when fetch fails", async () => {
    mocks.listEntities.mockRejectedValueOnce(new Error("Network error"));
    renderEntitiesPage();

    await waitFor(() => {
      expect(screen.getByText(/failed to load entities/i)).toBeInTheDocument();
    });
  });

  it("links entity labels to detail pages", async () => {
    renderEntitiesPage();

    await waitFor(() => {
      const links = screen.getAllByRole("link");
      expect(links.some((link) => link.getAttribute("href") === "/entities/$entityId/urn:zakhor:entity:react-flow")).toBe(true);
    });
  });
});

describe("EntityDetailPage", () => {
  beforeEach(() => {
    vi.useRealTimers();
  });

  it("fetches entity detail, decisions, and observations on mount", async () => {
    mocks.getEntity.mockResolvedValue(mockEntityDetail);
    mocks.getEntityDecisions.mockResolvedValue({ decisions: [], count: 0 });
    mocks.getEntityObservations.mockResolvedValue({ observations: [], count: 0 });

    renderEntityDetailPage();

    await waitFor(() => {
      expect(mocks.getEntity).toHaveBeenCalledWith("urn:zakhor:entity:react-flow");
      expect(mocks.getEntityDecisions).toHaveBeenCalledWith("urn:zakhor:entity:react-flow");
      expect(mocks.getEntityObservations).toHaveBeenCalledWith("urn:zakhor:entity:react-flow");
    });
  });

  it("displays entity label and uri in header", async () => {
    mocks.getEntity.mockResolvedValue(mockEntityDetail);
    mocks.getEntityDecisions.mockResolvedValue({ decisions: [], count: 0 });
    mocks.getEntityObservations.mockResolvedValue({ observations: [], count: 0 });

    renderEntityDetailPage();

    await waitFor(() => {
      expect(screen.getByText(/react flow/i)).toBeInTheDocument();
    });
  });

  it("renders type badges in card description", async () => {
    mocks.getEntity.mockResolvedValue(mockEntityDetail);
    mocks.getEntityDecisions.mockResolvedValue({ decisions: [], count: 0 });
    mocks.getEntityObservations.mockResolvedValue({ observations: [], count: 0 });

    renderEntityDetailPage();

    await waitFor(() => screen.getByText(/react flow/i));

    expect(screen.getByText("Technology")).toBeInTheDocument();
    expect(screen.getByText("Library")).toBeInTheDocument();
  });

  it("renders relationships tab", async () => {
    mocks.getEntity.mockResolvedValue(mockEntityDetail);
    mocks.getEntityDecisions.mockResolvedValue({ decisions: [], count: 0 });
    mocks.getEntityObservations.mockResolvedValue({ observations: [], count: 0 });

    renderEntityDetailPage();

    await waitFor(() => screen.getByRole("tab", { name: /relationships/i }));
  });

  it("renders decisions tab", async () => {
    mocks.getEntity.mockResolvedValue(mockEntityDetail);
    mocks.getEntityDecisions.mockResolvedValue({ decisions: [], count: 0 });
    mocks.getEntityObservations.mockResolvedValue({ observations: [], count: 0 });

    renderEntityDetailPage();

    await waitFor(() => screen.getByRole("tab", { name: /decisions/i }));
  });

  it("renders observations tab", async () => {
    mocks.getEntity.mockResolvedValue(mockEntityDetail);
    mocks.getEntityDecisions.mockResolvedValue({ decisions: [], count: 0 });
    mocks.getEntityObservations.mockResolvedValue({ observations: [], count: 0 });

    renderEntityDetailPage();

    await waitFor(() => screen.getByRole("tab", { name: /observations/i }));
  });

  it("displays relationships in table", async () => {
    mocks.getEntity.mockResolvedValue(mockEntityDetail);
    mocks.getEntityDecisions.mockResolvedValue({ decisions: [], count: 0 });
    mocks.getEntityObservations.mockResolvedValue({ observations: [], count: 0 });

    renderEntityDetailPage();

    await waitFor(() => screen.getByText(/relationships/i));

    expect(screen.getByText("dependsOn")).toBeInTheDocument();
  });

  it("shows related decisions with status badges", async () => {
    mocks.getEntity.mockResolvedValue(mockEntityDetail);
    mocks.getEntityDecisions.mockResolvedValue({
      decisions: [
        { id: "decision-1", title: "Test decision", status: "active" },
      ],
      count: 1,
    });
    mocks.getEntityObservations.mockResolvedValue({ observations: [], count: 0 });

    renderEntityDetailPage();

    await waitFor(() => expect(screen.getByText(/react flow/i)).toBeInTheDocument());
    await userEvent.click(screen.getByRole("tab", { name: /decisions/i }));

    await waitFor(() => expect(screen.getByText(/test decision/i)).toBeInTheDocument());
  });

  it("shows related observations in list", async () => {
    mocks.getEntity.mockResolvedValue(mockEntityDetail);
    mocks.getEntityDecisions.mockResolvedValue({ decisions: [], count: 0 });
    mocks.getEntityObservations.mockResolvedValue({
      observations: [
        { uri: "urn:zakhor:obs:1", text: "Test observation text" },
      ],
      count: 1,
    });

    renderEntityDetailPage();

    await waitFor(() => expect(screen.getByText(/react flow/i)).toBeInTheDocument());
    await userEvent.click(screen.getByRole("tab", { name: /observations/i }));

    await waitFor(() => expect(screen.getByText(/test observation text/i)).toBeInTheDocument());
  });

  it("displays source locations", async () => {
    mocks.getEntity.mockResolvedValue(mockEntityDetail);
    mocks.getEntityDecisions.mockResolvedValue({ decisions: [], count: 0 });
    mocks.getEntityObservations.mockResolvedValue({ observations: [], count: 0 });

    renderEntityDetailPage();

    await waitFor(() => screen.getByText(/source locations/i));

    expect(screen.getAllByText(/graph.ts/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/file:\/\/\/src\/utils\/graph.ts/i)).toBeInTheDocument();
  });

  it("shows error state when entity fetch fails", async () => {
    mocks.getEntity.mockRejectedValue(new Error("Network error"));
    mocks.getEntityDecisions.mockResolvedValue({ decisions: [], count: 0 });
    mocks.getEntityObservations.mockResolvedValue({ observations: [], count: 0 });

    renderEntityDetailPage();

    await waitFor(() => {
      expect(screen.getByText(/failed to load entity/i)).toBeInTheDocument();
    });
  });

  it("shows error state when decisions fetch fails", async () => {
    mocks.getEntity.mockResolvedValue(mockEntityDetail);
    mocks.getEntityDecisions.mockRejectedValue(new Error("Network error"));
    mocks.getEntityObservations.mockResolvedValue({ observations: [], count: 0 });

    renderEntityDetailPage();

    await waitFor(() => expect(screen.getByText(/react flow/i)).toBeInTheDocument());
    await userEvent.click(screen.getByRole("tab", { name: /decisions/i }));

    await waitFor(() => expect(screen.getByText(/failed to load decisions/i)).toBeInTheDocument());
  });

  it("shows error state when observations fetch fails", async () => {
    mocks.getEntity.mockResolvedValue(mockEntityDetail);
    mocks.getEntityDecisions.mockResolvedValue({ decisions: [], count: 0 });
    mocks.getEntityObservations.mockRejectedValue(new Error("Network error"));

    renderEntityDetailPage();

    await waitFor(() => expect(screen.getByText(/react flow/i)).toBeInTheDocument());
    await userEvent.click(screen.getByRole("tab", { name: /observations/i }));

    await waitFor(() => expect(screen.getByText(/failed to load observations/i)).toBeInTheDocument());
  });
});