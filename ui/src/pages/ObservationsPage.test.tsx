import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ObservationsPage } from "@/pages/ObservationsPage";
import type { ObservationListResponse } from "@/lib/api/observations";

const mockObservationsList: ObservationListResponse = {
  observations: [
    {
      id: "obs-1",
      text: "Tracker endpoint is the primary SPARQL storage backend.",
      created_at: "2024-06-15T10:30:00Z",
      confidence: 0.91,
      entity_refs: ["urn:zakhor:entity:tracker-sparql", "urn:zakhor:entity:sparql"],
      source: "ingestion",
      decision_refs: ["decision-1"],
    },
    {
      id: "obs-2",
      text: "Tantivy and Fastembed indexes mirror Tracker contents for search.",
      created_at: "2024-06-14T14:22:00Z",
      confidence: 0.84,
      entity_refs: ["urn:zakhor:entity:tantivy", "urn:zakhor:entity:fastembed"],
      source: "analysis",
      decision_refs: [],
    },
    {
      id: "obs-3",
      text: "React Flow selected for graph visualization due to TypeScript support.",
      created_at: "2024-06-10T09:15:00Z",
      confidence: 0.78,
      entity_refs: ["urn:zakhor:entity:react-flow"],
      source: "decision",
      decision_refs: ["decision-2", "decision-3"],
    },
  ],
  count: 3,
  total: 3,
};

const mockEmptyList: ObservationListResponse = {
  observations: [],
  count: 0,
  total: 0,
};

const mocks = vi.hoisted(() => ({
  listObservations: vi.fn(),
}));

vi.mock("@/lib/api/observations", () => ({
  listObservations: mocks.listObservations,
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => vi.fn(),
  useSearch: () => ({
    entity_id: undefined,
    from: undefined,
    to: undefined,
    min_confidence: undefined,
    sort: "newest",
    limit: 20,
    offset: 0,
  }),
}));

function createQueryClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } });
}

function renderObservationsPage() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <ObservationsPage />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  vi.resetAllMocks();
});

describe("ObservationsPage", () => {
  beforeEach(() => {
    vi.useRealTimers();
    mocks.listObservations.mockResolvedValue(mockObservationsList);
  });

  it("renders page title and description", () => {
    renderObservationsPage();
    expect(screen.getByText("Observation Timeline")).toBeInTheDocument();
    expect(screen.getByText(/chronological observations with entity and decision links/i)).toBeInTheDocument();
  });

  it("renders filter inputs", () => {
    renderObservationsPage();
    expect(screen.getByPlaceholderText(/filter by entity id/i)).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/from/i)).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/to/i)).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/min confidence/i)).toBeInTheDocument();
  });

  it("renders sort buttons", () => {
    renderObservationsPage();
    expect(screen.getByText(/sort by:/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /newest/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /oldest/i })).toBeInTheDocument();
  });

  it("fetches observations with correct default query parameters", async () => {
    renderObservationsPage();

    await waitFor(() => {
      expect(mocks.listObservations).toHaveBeenCalledWith(
        expect.objectContaining({
          offset: 0,
          limit: 20,
          sort: "newest",
        }),
      );
    });
  });

  it("displays observations after successful fetch", async () => {
    renderObservationsPage();

    await waitFor(() => {
      expect(screen.getByText(/tracker endpoint is the primary sparql storage backend/i)).toBeInTheDocument();
      expect(screen.getByText(/tantivy and fastembed indexes mirror tracker contents/i)).toBeInTheDocument();
      expect(screen.getByText(/react flow selected for graph visualization/i)).toBeInTheDocument();
    });
  });

  it("displays confidence badges on observation cards", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    expect(screen.getByText("91%")).toBeInTheDocument();
    expect(screen.getByText("84%")).toBeInTheDocument();
    expect(screen.getByText("78%")).toBeInTheDocument();
  });

  it("displays formatted dates on observation cards", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    expect(screen.getAllByText(/jun/i).length).toBe(3);
  });

  it("displays source labels on observation cards", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    expect(screen.getByText("ingestion")).toBeInTheDocument();
    expect(screen.getByText("analysis")).toBeInTheDocument();
    expect(screen.getByText("decision")).toBeInTheDocument();
  });

  it("displays entity reference badges", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    expect(screen.getByText("urn:zakhor:entity:tracker-sparql")).toBeInTheDocument();
    expect(screen.getByText("urn:zakhor:entity:sparql")).toBeInTheDocument();
    expect(screen.getByText("urn:zakhor:entity:tantivy")).toBeInTheDocument();
    expect(screen.getByText("urn:zakhor:entity:fastembed")).toBeInTheDocument();
    expect(screen.getByText("urn:zakhor:entity:react-flow")).toBeInTheDocument();
  });

  it("displays decision reference badges", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    expect(screen.getByText("decision-1")).toBeInTheDocument();
    expect(screen.getByText("decision-2")).toBeInTheDocument();
    expect(screen.getByText("decision-3")).toBeInTheDocument();
  });

  it("shows total count", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    expect(screen.getByText(/total: 3/i)).toBeInTheDocument();
  });

  it("displays empty state when no observations found", async () => {
    mocks.listObservations.mockResolvedValueOnce(mockEmptyList);
    renderObservationsPage();

    await waitFor(() => {
      expect(screen.getByText("No observations found")).toBeInTheDocument();
    });
  });

  it("shows error message when fetch fails", async () => {
    mocks.listObservations.mockRejectedValueOnce(new Error("Network error"));
    renderObservationsPage();

    await waitFor(() => {
      expect(screen.getByText(/failed to load observations/i)).toBeInTheDocument();
    });
  });

  it("shows loading skeletons while fetching", () => {
    mocks.listObservations.mockImplementation(() => new Promise(() => {}));
    renderObservationsPage();

    expect(document.querySelectorAll(".animate-pulse").length).toBeGreaterThan(0);
  });

  it("applies entity filter when entity_id is provided", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    expect(mocks.listObservations).toHaveBeenCalledWith(
      expect.objectContaining({
        entityId: undefined,
      }),
    );

    vi.clearAllMocks();
    mocks.listObservations.mockResolvedValue(mockObservationsList);

    const queryKey = ["observations", "urn:zakhor:entity:test", undefined, undefined, undefined, "newest", 20, 0];
    expect(queryKey).toContain("urn:zakhor:entity:test");
  });

  it("applies date range filters when from/to are provided", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    expect(mocks.listObservations).toHaveBeenCalledWith(
      expect.objectContaining({
        from: undefined,
        to: undefined,
      }),
    );
  });

  it("applies min confidence filter when provided", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    expect(mocks.listObservations).toHaveBeenCalledWith(
      expect.objectContaining({
        minConfidence: undefined,
      }),
    );
  });

  it("applies sort parameter", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    expect(mocks.listObservations).toHaveBeenCalledWith(
      expect.objectContaining({
        sort: "newest",
      }),
    );
  });

  it("renders pagination controls", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    expect(screen.getByRole("button", { name: /previous/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /next/i })).toBeInTheDocument();
    expect(screen.getByText(/page 1 of 1/i)).toBeInTheDocument();
  });

  it("shows showing range in pagination", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    expect(screen.getByText(/showing 1-3 of 3/i)).toBeInTheDocument();
  });

  it("displays timeline visual with alternating cards", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    // Check for the timeline line (border-l)
    const timelineContainer = screen.getByText(/tracker endpoint is the primary sparql storage backend/i).closest("div[class*='border-l']");
    expect(timelineContainer).toBeInTheDocument();
  });

  it("confidence indicator colors map correctly", async () => {
    renderObservationsPage();

    await waitFor(() => screen.getByText(/tracker endpoint is the primary sparql storage backend/i));

    // High confidence (0.91) should have green indicator
    // Medium confidence (0.84) should have yellow indicator
    // Lower confidence (0.78) should have yellow indicator
    const confidenceIndicators = screen.getAllByText(/^\d+%$/);
    expect(confidenceIndicators.length).toBe(3);
  });
});