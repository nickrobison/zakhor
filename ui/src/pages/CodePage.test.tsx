import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CodeResponse } from "@/lib/api/code";
import { CodePage } from "@/pages/CodePage";

const mocks = vi.hoisted(() => ({
  getCodeReferences: vi.fn(),
}));

const mockCode: CodeResponse = {
  repositories: [
    { name: "zakhor", url: "https://github.com/example/zakhor", description: "Knowledge memory server" },
  ],
  files: [
    { path: "zakhor/src/main.rs", repository: "zakhor", language: "rust" },
    { path: "zakhor/ui/src/pages/CodePage.tsx", repository: "zakhor", language: "typescript" },
  ],
  symbols: [
    { name: "CodePage", kind: "component", file_path: "zakhor/ui/src/pages/CodePage.tsx", line: 12 },
    { name: "serve_api", kind: "function", file_path: "zakhor/src/main.rs", line: 48 },
  ],
};

vi.mock("@/lib/api/code", () => ({
  getCodeReferences: mocks.getCodeReferences,
}));

function createQueryClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

function renderCodePage() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <CodePage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  mocks.getCodeReferences.mockResolvedValue(mockCode);
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("CodePage", () => {
  it("renders entity search controls", () => {
    renderCodePage();

    expect(screen.getByLabelText(/entity id or search query/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /search/i })).toBeDisabled();
  });

  it("loads code references grouped by repository", async () => {
    renderCodePage();

    fireEvent.change(screen.getByLabelText(/entity id or search query/i), { target: { value: "react-flow" } });
    fireEvent.click(screen.getByRole("button", { name: /search/i }));

    await waitFor(() => {
      expect(mocks.getCodeReferences).toHaveBeenCalledWith("react-flow");
      expect(screen.getByText("zakhor")).toBeInTheDocument();
      expect(screen.getByText("2 files")).toBeInTheDocument();
      expect(screen.getByText("2 symbols")).toBeInTheDocument();
      expect(screen.getByText("CodePage")).toBeInTheDocument();
      expect(screen.getByText("serve_api")).toBeInTheDocument();
    });
  });

  it("switches between repository, file, and symbol tabs", async () => {
    renderCodePage();

    fireEvent.change(screen.getByLabelText(/entity id or search query/i), { target: { value: "react-flow" } });
    fireEvent.click(screen.getByRole("button", { name: /search/i }));

    await waitFor(() => expect(screen.getByText("zakhor")).toBeInTheDocument());

    fireEvent.click(screen.getByRole("tab", { name: /files/i }));
    expect(screen.getByText("zakhor/src/main.rs")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: /symbols/i }));
    expect(screen.getByText("CodePage")).toBeInTheDocument();
    expect(screen.getByText("serve_api")).toBeInTheDocument();
  });

  it("shows empty state when no results are returned", async () => {
    mocks.getCodeReferences.mockResolvedValue({ repositories: [], files: [], symbols: [] });
    renderCodePage();

    fireEvent.change(screen.getByLabelText(/entity id or search query/i), { target: { value: "missing" } });
    fireEvent.click(screen.getByRole("button", { name: /search/i }));

    await waitFor(() => expect(screen.getByText(/no code references found/i)).toBeInTheDocument());
  });
});
