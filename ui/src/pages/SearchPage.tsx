import { Component, useEffect, useMemo, useState, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { getAdminStatus } from "@/lib/api/admin";
import { searchHybrid, type SearchResponse } from "@/lib/api/search";

const PAGE_SIZE = 10;
const DEFAULT_LIMIT = 20;
const searchModes = ["hybrid", "lexical", "semantic"] as const;
const resultTabs = [
  { value: "all", label: "All" },
  { value: "decisions", label: "Decisions" },
  { value: "entities", label: "Entities" },
  { value: "observations", label: "Observations" },
] as const;
const modeLabels: Record<SearchMode, string> = {
  hybrid: "Hybrid",
  lexical: "Full text",
  semantic: "Semantic",
};

type SearchMode = (typeof searchModes)[number];
type ActiveTab = "all" | "decisions" | "entities" | "observations";
type SearchResultType = ActiveTab | "unknown";
type SearchPageErrorState = { hasError: boolean };

export function SearchPage() {
  return (
    <SearchPageErrorBoundary>
      <SearchPageContent />
    </SearchPageErrorBoundary>
  );
}

class SearchPageErrorBoundary extends Component<{ children: ReactNode }, SearchPageErrorState> {
  state: SearchPageErrorState = { hasError: false };

  static getDerivedStateFromError(): SearchPageErrorState {
    return { hasError: true };
  }

  componentDidCatch(error: Error): void {
    console.error(error);
  }

  render() {
    if (this.state.hasError) {
      return (
        <Card>
          <CardHeader>
            <CardTitle>Search failed</CardTitle>
            <CardDescription>The search UI encountered an unexpected error.</CardDescription>
          </CardHeader>
          <CardContent>
            <Button type="button" variant="outline" onClick={() => this.setState({ hasError: false })}>
              Try again
            </Button>
          </CardContent>
        </Card>
      );
    }

    return this.props.children;
  }
}

function SearchPageContent() {
  const navigate = useNavigate();
  const searchParams = useSearch({ from: "/search" });
  const [draftQuery, setDraftQuery] = useState(searchParams.q ?? "");
  const [activeTab, setActiveTab] = useState<Exclude<SearchResultType, "unknown">>(searchParams.type ?? "all");
  const [mode, setMode] = useState<SearchMode>(normalizeMode(searchParams.mode ?? "hybrid"));
  const [limit, setLimit] = useState(String(searchParams.limit ?? DEFAULT_LIMIT));
  const [page, setPage] = useState(searchParams.page ?? 1);

  useEffect(() => setDraftQuery(searchParams.q ?? ""), [searchParams.q]);
  useEffect(() => setActiveTab(searchParams.type ?? "all"), [searchParams.type]);
  useEffect(() => setMode(normalizeMode(searchParams.mode ?? "hybrid")), [searchParams.mode]);
  useEffect(() => setLimit(String(searchParams.limit ?? DEFAULT_LIMIT)), [searchParams.limit]);
  useEffect(() => setPage(searchParams.page ?? 1), [searchParams.page]);

  const limitNumber = clampLimit(parseLimit(limit));
  const queryText = draftQuery.trim();
  const status = useQuery({ queryKey: ["admin-status", "search"], queryFn: getAdminStatus, retry: false });
  const search = useQuery({
    queryKey: ["search", queryText, limitNumber, mode],
    queryFn: () => searchHybrid(queryText, limitNumber, mode),
    enabled: queryText.length > 0,
    retry: false,
  });

  const filteredResults = useMemo<SearchResponse["results"]>(() => {
    const results = search.data?.results ?? [];
    if (activeTab === "all") return results;
    return results.filter((result) => inferResultType(result.id) === activeTab);
  }, [activeTab, search.data?.results]);

  const pageCount = Math.max(1, Math.ceil(filteredResults.length / PAGE_SIZE));
  const safePage = clampPage(page, pageCount);
  const pageResults = filteredResults.slice((safePage - 1) * PAGE_SIZE, safePage * PAGE_SIZE);

  useEffect(() => {
    if (page !== safePage) setPage(safePage);
  }, [page, safePage]);

  function updateRoute(nextQuery = queryText, nextType = activeTab, nextMode = mode, nextLimit = limitNumber, nextPage = safePage) {
    void navigate({
      to: "/search",
      search: {
        q: nextQuery || undefined,
        type: nextType,
        mode: nextMode,
        limit: nextLimit,
        page: nextPage,
      },
    });
  }

  function submitSearch(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void updateRoute(draftQuery.trim(), activeTab, mode, limitNumber, 1);
  }

  function handleTabChange(value: string) {
    const nextTab = normalizeTab(value);
    setActiveTab(nextTab);
    void updateRoute(queryText, nextTab, mode, limitNumber, 1);
  }

  function handleModeChange(nextMode: SearchMode) {
    setMode(nextMode);
    void updateRoute(queryText, activeTab, nextMode, limitNumber, 1);
  }

  function handlePageChange(nextPage: number) {
    const safeNextPage = clampPage(nextPage, pageCount);
    setPage(safeNextPage);
    void updateRoute(queryText, activeTab, mode, limitNumber, safeNextPage);
  }

  return (
    <section className="space-y-6">
      <div>
        <h1 className="text-3xl font-semibold tracking-tight">Search</h1>
        <p className="mt-2 text-muted-foreground">Hybrid, full-text, and semantic search across the local knowledge graph.</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Search observations and decisions</CardTitle>
          <CardDescription>Connects to GET /api/v1/search with reciprocal-rank fusion by default.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <form className="flex flex-col gap-3 lg:flex-row" onSubmit={submitSearch}>
            <div className="flex-1 space-y-2">
              <label className="text-sm font-medium" htmlFor="search-query">
                Search query
              </label>
              <Input
                id="search-query"
                value={draftQuery}
                onChange={(event) => setDraftQuery(event.target.value)}
                placeholder="Search query"
              />
            </div>
            <div className="space-y-2 lg:w-32">
              <label className="text-sm font-medium" htmlFor="search-limit">
                Limit
              </label>
              <Input
                id="search-limit"
                type="number"
                min="1"
                max="100"
                value={limit}
                onChange={(event) => setLimit(event.target.value)}
              />
            </div>
            <div className="flex items-end">
              <Button type="submit" disabled={!draftQuery.trim()}>
                Search
              </Button>
            </div>
          </form>

          <div className="space-y-2">
            <p className="text-sm font-medium">Ranking</p>
            <div className="flex flex-wrap gap-2" role="group" aria-label="Search ranking mode">
              {searchModes.map((nextMode) => (
                <Button
                  key={nextMode}
                  type="button"
                  variant={mode === nextMode ? "default" : "outline"}
                  onClick={() => handleModeChange(nextMode)}
                  aria-pressed={mode === nextMode}
                >
                  {modeLabels[nextMode]}
                </Button>
              ))}
            </div>
          </div>

          {status.isLoading ? (
            <Skeleton className="h-4 w-48" />
          ) : status.isError ? (
            <p className="text-sm text-destructive">Index status is unavailable.</p>
          ) : status.data ? (
            <p className="text-sm text-muted-foreground">
              Indexes: {status.data.indexes_available ? "available" : "pending"} · Lexical docs:{" "}
              {status.data.lexical_docs.toLocaleString()} · Semantic vectors: {status.data.semantic_vectors.toLocaleString()}
            </p>
          ) : null}

          {search.isError && <p className="text-sm text-destructive">Search failed. Start the Rust API and rebuild indexes first.</p>}
          {search.data?.warning && <p className="text-sm text-muted-foreground">{search.data.warning}</p>}

          <div role="tablist" aria-label="Search result type" className="space-y-2">
            <div className="flex flex-wrap gap-2">
              {resultTabs.map((tab) => (
                <Button
                  key={tab.value}
                  type="button"
                  variant={activeTab === tab.value ? "default" : "outline"}
                  role="tab"
                  aria-selected={activeTab === tab.value}
                  onClick={() => handleTabChange(tab.value)}
                >
                  {tab.label}
                </Button>
              ))}
            </div>

            <div role="tabpanel" aria-label={`${resultTypeLabel(activeTab)} results`}>
              <div className="mt-4 space-y-4">
                <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                  <p className="text-sm text-muted-foreground" aria-live="polite">
                    {queryText ? `${filteredResults.length} matching results` : "Enter a search query to see results."}
                  </p>
                  <p className="text-sm text-muted-foreground">Page {safePage} of {pageCount}</p>
                </div>

                {search.isLoading ? (
                  <div className="space-y-3">
                    <Skeleton className="h-20 w-full" />
                    <Skeleton className="h-20 w-full" />
                    <Skeleton className="h-20 w-full" />
                  </div>
                ) : pageResults.length === 0 ? (
                  <EmptyResults query={queryText} />
                ) : (
                  <div className="space-y-3">
                    {pageResults.map((result) => (
                      <SearchResultCard key={result.id} result={result} />
                    ))}
                    <Pagination currentPage={safePage} pageCount={pageCount} onPageChange={handlePageChange} />
                  </div>
                )}
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </section>
  );
}

function EmptyResults({ query }: { query: string }) {
  return (
    <Card>
      <CardContent className="pt-6">
        <p className="text-sm text-muted-foreground">
          {query ? "No results found for this query and filter." : "Enter a search query to see hybrid search results."}
        </p>
      </CardContent>
    </Card>
  );
}

function SearchResultCard({ result }: { result: SearchResponse["results"][number] }) {
  const type = inferResultType(result.id);

  return (
    <article className="rounded-md border p-4 transition-colors hover:bg-accent">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0 space-y-2">
          <p className="break-all text-sm font-medium">{result.id}</p>
          <p className="text-sm text-muted-foreground">Score {result.score.toFixed(4)}</p>
          <p className="text-sm text-muted-foreground">Snippet and timestamp are not returned by the current search payload.</p>
        </div>
        <ResultTypeBadge type={type} />
      </div>
    </article>
  );
}

function ResultTypeBadge({ type }: { type: SearchResultType }) {
  const label = resultTypeLabel(type);
  const variant = type === "unknown" || type === "all" ? "secondary" : "outline";

  return <Badge variant={variant}>{label}</Badge>;
}

function Pagination({ currentPage, pageCount, onPageChange }: { currentPage: number; pageCount: number; onPageChange: (page: number) => void }) {
  return (
    <div className="flex items-center justify-between gap-2">
      <Button type="button" variant="outline" disabled={currentPage <= 1} onClick={() => onPageChange(currentPage - 1)}>
        Previous
      </Button>
      <p className="text-sm text-muted-foreground">
        Page {currentPage} of {pageCount}
      </p>
      <Button type="button" variant="outline" disabled={currentPage >= pageCount} onClick={() => onPageChange(currentPage + 1)}>
        Next
      </Button>
    </div>
  );
}

function inferResultType(id: string): SearchResultType {
  const normalized = id.toLowerCase();

  if (normalized.includes("decision") || normalized.includes("outcome")) return "decisions";
  if (normalized.includes("entity") || normalized.includes("component") || normalized.includes("technology")) return "entities";
  if (normalized.includes("observation") || normalized.includes("informationelement")) return "observations";
  return "unknown";
}

function resultTypeLabel(type: SearchResultType) {
  switch (type) {
    case "all":
      return "All";
    case "decisions":
      return "Decision";
    case "entities":
      return "Entity";
    case "observations":
      return "Observation";
    case "unknown":
      return "Result";
    default:
      return assertNever(type);
  }
}

function normalizeTab(value: string): ActiveTab {
  switch (value) {
    case "all":
    case "decisions":
    case "entities":
    case "observations":
      return value;
    default:
      return "all";
  }
}

function normalizeMode(value: string): SearchMode {
  switch (value) {
    case "hybrid":
    case "lexical":
    case "semantic":
      return value;
    default:
      return "hybrid";
  }
}

function parseLimit(value: string) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return DEFAULT_LIMIT;
  return Math.round(parsed);
}

function clampLimit(value: number) {
  return Math.min(100, Math.max(1, value));
}

function clampPage(value: number, max: number) {
  return Math.min(max, Math.max(1, value));
}

function assertNever(value: never): never {
  throw new Error(`Unexpected value: ${String(value)}`);
}
