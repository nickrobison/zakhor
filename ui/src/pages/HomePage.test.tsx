import type { ReactNode } from "react";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HomePage } from "@/pages/HomePage";

vi.mock("@/lib/api", () => ({
  fetchHealth: vi.fn().mockResolvedValue({ status: "ok" }),
}));

vi.mock("@/lib/api/admin", () => ({
  getAdminStatus: vi.fn().mockResolvedValue({
    rebuild_in_progress: false,
    lexical_docs: 42,
    semantic_vectors: 42,
    indexes_available: true,
  }),
}));

vi.mock("@/lib/api/decisions", () => ({
  listDecisions: vi.fn().mockResolvedValue({
    decisions: [{ id: "decision-1", title: "Use React Flow", status: "active", modified: "2026-06-19T00:00:00Z" }],
    count: 1,
    total: 12,
  }),
}));

vi.mock("@/lib/api/entities", () => ({
  listEntities: vi.fn().mockResolvedValue({
    entities: [{ uri: "entity-1", label: "React Flow" }],
    count: 7,
  }),
}));

vi.mock("@/lib/api/observations", () => ({
  listObservations: vi.fn().mockResolvedValue({
    observations: [{ id: "obs-1", text: "Tracker endpoint is the primary storage backend.", created_at: "2026-06-19T00:00:00Z" }],
    count: 1,
    total: 3,
  }),
}));

vi.mock("@tanstack/react-router", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@tanstack/react-router")>();
  return {
    ...actual,
    Link: ({ children, to }: { children: ReactNode; to: string }) => <a href={to}>{children}</a>,
    useNavigate: () => vi.fn(),
  };
});

function createQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
}

function renderHomePage() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <HomePage />
    </QueryClientProvider>,
  );
}

afterEach(() => cleanup());

describe("HomePage", () => {
  it("renders dashboard title", () => {
    renderHomePage();
    expect(screen.getByRole("heading", { name: "Zakhor Web UI" })).toBeInTheDocument();
  });

  it("renders system stats and quick search", async () => {
    renderHomePage();

    expect((await screen.findAllByText("Decisions")).length).toBeGreaterThan(0);
    expect(await screen.findByText("Ready")).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/search decisions, entities, and observations/i)).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText("12")).toBeInTheDocument();
      expect(screen.getByText("7")).toBeInTheDocument();
      expect(screen.getByText("3")).toBeInTheDocument();
    });
  });
});
