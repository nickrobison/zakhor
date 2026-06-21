import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SearchPage } from "@/pages/SearchPage";

const mocks = vi.hoisted(() => ({
  searchHybrid: vi.fn().mockResolvedValue({
    results: [
      { id: "urn:zakhor:decision:react-flow", score: 0.82 },
      { id: "urn:zakhor:entity:react-flow", score: 0.74 },
      { id: "urn:zakhor:observation:react-flow", score: 0.69 },
    ],
    count: 3,
  }),
  getAdminStatus: vi.fn().mockResolvedValue({
    rebuild_in_progress: false,
    lexical_docs: 42,
    semantic_vectors: 42,
    indexes_available: true,
  }),
}));

vi.mock("@/lib/api/admin", () => ({
  getAdminStatus: mocks.getAdminStatus,
}));

vi.mock("@/lib/api/search", () => ({
  searchHybrid: mocks.searchHybrid,
}));

vi.mock("@tanstack/react-router", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@tanstack/react-router")>();
  return {
    ...actual,
    useNavigate: () => vi.fn(),
    useSearch: () => ({ q: "", type: "all", mode: "hybrid", limit: 20, page: 1 }),
  };
});

function createQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
}

function renderSearchPage() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <SearchPage />
    </QueryClientProvider>,
  );
}

afterEach(() => cleanup());

describe("SearchPage", () => {
  it("renders search controls", () => {
    renderSearchPage();

    expect(screen.getByLabelText(/search query/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /search/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /semantic/i })).toBeInTheDocument();
  });

  it("runs a search and displays filtered results", async () => {
    renderSearchPage();

    fireEvent.change(screen.getByLabelText(/search query/i), { target: { value: "react flow" } });
    fireEvent.submit(screen.getByRole("button", { name: /search/i }));

    expect(await screen.findByText("3 matching results")).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByText("urn:zakhor:decision:react-flow")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("tab", { name: /decisions/i }));

    await waitFor(() => {
      expect(screen.getByText("1 matching results")).toBeInTheDocument();
      expect(screen.getByText("urn:zakhor:decision:react-flow")).toBeInTheDocument();
      expect(screen.queryByText("urn:zakhor:entity:react-flow")).not.toBeInTheDocument();
    });
  });
});
