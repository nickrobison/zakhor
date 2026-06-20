import type * as React from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ConfidenceIndicator,
  DataTable,
  EmptyState,
  EntityTag,
  ErrorBoundary,
  LayoutShell,
  LoadingSkeleton,
  PaginationControls,
  SearchInput,
  SidebarNavigation,
  StatusBadge,
  TopBar,
} from "@/components/shared";

const mocks = vi.hoisted(() => ({
  Link: ({ children, to, params }: { children: React.ReactNode; to: string; params?: { entityId?: string } }) => (
    <a href={params?.entityId ? `${to}/${params.entityId}` : to}>{children}</a>
  ),
}));

vi.mock("@tanstack/react-router", () => ({
  Link: mocks.Link,
}));

function ThrowingChild(): React.ReactElement {
  throw new Error("boom");
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("shared UI components", () => {
  it("renders layout shell with sidebar and top bar", () => {
    render(<LayoutShell title="Zakhor" subtitle="Knowledge graph memory console">Content</LayoutShell>);

    expect(screen.getAllByText("Zakhor")).toHaveLength(2);
    expect(screen.getByText("Knowledge graph memory console")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /decisions/i })).toHaveAttribute("href", "/decisions");
    expect(screen.getByText("Content")).toBeInTheDocument();
  });

  it("renders sidebar navigation links", () => {
    render(<SidebarNavigation />);

    expect(screen.getByRole("navigation", { name: "Primary navigation" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /admin/i })).toHaveAttribute("href", "/admin");
  });

  it("renders top bar with quick search and actions", () => {
    const onSearchChange = vi.fn();
    render(
      <TopBar title="Search" searchValue="tracker" onSearchChange={onSearchChange}>
        <button type="button">Action</button>
      </TopBar>,
    );

    const searchboxes = screen.getAllByRole("textbox");
    expect(searchboxes[0]).toHaveValue("tracker");
    fireEvent.change(searchboxes[0], { target: { value: "entity" } });
    expect(onSearchChange).toHaveBeenCalledWith("entity");
    expect(screen.getByRole("button", { name: "Action" })).toBeInTheDocument();
  });

  it("renders search input", () => {
    const onChange = vi.fn();
    render(<SearchInput value="abc" placeholder="Search decisions" onChange={onChange} />);

    fireEvent.change(screen.getByPlaceholderText("Search decisions"), { target: { value: "abcd" } });
    expect(onChange).toHaveBeenCalledWith("abcd");
  });

  it("renders status badge", () => {
    render(<StatusBadge status="warning" />);

    expect(screen.getByText("warning")).toBeInTheDocument();
  });

  it("renders confidence indicator", () => {
    render(<ConfidenceIndicator value={87} />);

    expect(screen.getByText("87%")).toBeInTheDocument();
    expect(screen.getByLabelText("Confidence: 87%")).toBeInTheDocument();
  });

  it("renders entity tag link", () => {
    render(<EntityTag uri="urn:zakhor:entity:react" label="React" />);

    expect(screen.getByRole("link", { name: "React" })).toHaveAttribute("href", "/entities/$entityId/urn:zakhor:entity:react");
  });

  it("renders empty state with optional action", () => {
    const onAction = vi.fn();
    render(<EmptyState title="No results" description="Try another search." actionLabel="Retry" onAction={onAction} />);

    expect(screen.getByText("No results")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(onAction).toHaveBeenCalledTimes(1);
  });

  it("renders error boundary fallback", () => {
    const onRetry = vi.fn();
    render(
      <ErrorBoundary onRetry={onRetry} title="Oops" description="Failed">
        <ThrowingChild />
      </ErrorBoundary>,
    );

    expect(screen.getByText("Oops")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("renders loading skeletons", () => {
    render(<LoadingSkeleton count={2} />);

    expect(screen.getAllByTestId("skeleton")).toHaveLength(2);
  });

  it("renders pagination controls", () => {
    const onPageChange = vi.fn();
    render(<PaginationControls currentPage={2} pageCount={5} onPageChange={onPageChange} pageSize={25} total={100} />);

    expect(screen.getByText((content) => content.includes("Showing 26-50 of 100"))).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(onPageChange).toHaveBeenCalledWith(3);
  });

  it("renders empty pagination controls", () => {
    render(<PaginationControls currentPage={1} pageCount={1} onPageChange={vi.fn()} pageSize={25} total={0} />);

    expect(screen.getByText((content) => content.includes("Showing 0-0 of 0"))).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Previous" })).toBeDisabled();
  });

  it("renders data table rows and empty state", () => {
    render(
      <DataTable
        columns={[{ header: "Name", accessorKey: "name" }]}
        data={[{ name: "Alpha" }]}
      />,
    );

    expect(screen.getByText("Alpha")).toBeInTheDocument();
  });
});
