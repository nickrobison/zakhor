import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AdminStatus } from "@/lib/api/admin";
import { AdminPage } from "@/pages/AdminPage";

const mocks = vi.hoisted(() => ({
  getAdminStatus: vi.fn(),
  getHealth: vi.fn(),
  rebuildIndexes: vi.fn(),
}));

const mockStatus: AdminStatus = {
  rebuild_in_progress: false,
  lexical_docs: 120,
  semantic_vectors: 80,
  indexes_available: true,
  last_rebuild_at_ms: 1_780_000_000_000,
};

vi.mock("@/lib/api/admin", () => ({
  getAdminStatus: mocks.getAdminStatus,
  rebuildIndexes: mocks.rebuildIndexes,
}));

vi.mock("@/lib/api/health", () => ({
  getHealth: mocks.getHealth,
}));

function createQueryClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

function renderAdminPage() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <AdminPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  mocks.getHealth.mockResolvedValue({ status: "ok" });
  mocks.getAdminStatus.mockResolvedValue(mockStatus);
  mocks.rebuildIndexes.mockResolvedValue({ status: "accepted", message: "rebuild started" });
  vi.spyOn(window, "confirm").mockReturnValue(true);
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("AdminPage", () => {
  it("renders admin status cards", async () => {
    renderAdminPage();

    await waitFor(() => expect(screen.getByText("Tracker")).toBeInTheDocument());
    expect(screen.getByText("Indexes")).toBeInTheDocument();
    expect(screen.getByText("Lexical docs")).toBeInTheDocument();
    expect(screen.getByText("Semantic vectors")).toBeInTheDocument();
  });

  it("shows detailed index status after loading", async () => {
    renderAdminPage();

    await waitFor(() => expect(screen.getAllByText("120").length).toBeGreaterThanOrEqual(2));
    await waitFor(() => expect(screen.getAllByText("80").length).toBeGreaterThanOrEqual(2));
    await waitFor(() => expect(screen.getAllByText("Indexes available: yes").length).toBeGreaterThanOrEqual(1));
  });

  it("confirms and triggers rebuild", async () => {
    renderAdminPage();

    await waitFor(() => expect(screen.getAllByText("Indexes available: yes").length).toBeGreaterThanOrEqual(1));
    fireEvent.click(screen.getAllByRole("button", { name: /rebuild indexes/i }).at(-1)!);

    await waitFor(() => {
      expect(window.confirm).toHaveBeenCalledWith("Rebuild lexical and semantic indexes? This may take some time.");
      expect(mocks.rebuildIndexes).toHaveBeenCalledTimes(1);
    });
  });

  it("disables rebuild while rebuild is in progress", async () => {
    mocks.getAdminStatus.mockResolvedValue({ ...mockStatus, rebuild_in_progress: true });
    renderAdminPage();

    expect(await screen.findByText("A rebuild is already in progress.")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: /rebuild indexes/i }).at(-1)).toBeDisabled();
  });
});
