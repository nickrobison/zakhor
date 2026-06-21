import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { listObservations, type ObservationSort, type ObservationSummary } from "@/lib/api/observations";

const DEFAULT_LIMIT = 20;
const SORT_OPTIONS: ObservationSort[] = ["newest", "oldest"];

function normalizeSort(value: string): ObservationSort {
  if (SORT_OPTIONS.includes(value as ObservationSort)) return value as ObservationSort;
  return "newest";
}

function formatDate(dateString: string | null | undefined): string {
  if (!dateString) return "Unknown date";
  try {
    return new Date(dateString).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return "Invalid date";
  }
}

function getConfidenceColor(confidence: number | null | undefined): string {
  if (confidence === null || confidence === undefined) return "bg-muted";
  if (confidence >= 0.8) return "bg-green-500";
  if (confidence >= 0.6) return "bg-yellow-500";
  if (confidence >= 0.4) return "bg-orange-500";
  return "bg-red-500";
}

function ObservationCard({ observation, index, total }: { observation: ObservationSummary; index: number; total: number }) {
  const isEven = index % 2 === 0;

  return (
    <div className="relative flex gap-4">
      <div className="absolute left-1/2 top-0 -translate-x-1/2 w-3 h-3 rounded-full border-4 border-background z-10" style={{ backgroundColor: getConfidenceColor(observation.confidence) }} />
      <div className={`w-1/2 ${isEven ? "pr-4 text-right" : "pl-4 ml-auto"}`}>
        <Card className="transition-colors hover:bg-accent cursor-pointer">
          <CardHeader className="pb-2">
            <div className="flex items-start justify-between gap-2">
              <CardTitle className="text-base">{observation.text.slice(0, 120)}{observation.text.length > 120 ? "…" : ""}</CardTitle>
              {observation.confidence !== null && observation.confidence !== undefined && (
                <Badge variant="secondary" className="flex-shrink-0">
                  {(observation.confidence * 100).toFixed(0)}%
                </Badge>
              )}
            </div>
            <CardDescription className="text-xs">
              {formatDate(observation.created_at)}
              {observation.source && <span className="mx-1">·</span>}
              {observation.source && <span className="font-mono text-xs">{observation.source}</span>}
            </CardDescription>
          </CardHeader>
          <CardContent className="pt-0">
            <div className="flex flex-wrap gap-1">
              {observation.entity_refs?.slice(0, 3).map((entityRef, i) => (
                <Badge key={i} variant="outline" className="text-xs">
                  {entityRef}
                </Badge>
              ))}
              {observation.entity_refs && observation.entity_refs.length > 3 && (
                <Badge variant="outline" className="text-xs">
                  +{observation.entity_refs.length - 3} more
                </Badge>
              )}
              {observation.decision_refs?.slice(0, 2).map((decisionRef, i) => (
                <Badge key={i} variant="default" className="text-xs">
                  {decisionRef}
                </Badge>
              ))}
              {observation.decision_refs && observation.decision_refs.length > 2 && (
                <Badge variant="default" className="text-xs">
                  +{observation.decision_refs.length - 2} more
                </Badge>
              )}
            </div>
          </CardContent>
        </Card>
      </div>
      {index < total - 1 && (
        <div className="absolute left-1/2 top-8 -translate-x-1/2 w-0.5 h-full bg-border" />
      )}
    </div>
  );
}

type ObservationSearchParams = {
  entity_id?: string;
  from?: string;
  to?: string;
  min_confidence?: number;
  sort?: ObservationSort;
  offset?: number;
  limit?: number;
};

export function ObservationsPage() {
  const navigate = useNavigate();
  const searchParams = useSearch({ from: "/observations" }) as ObservationSearchParams;

  const entityId = searchParams.entity_id;
  const from = searchParams.from;
  const to = searchParams.to;
  const minConfidence = searchParams.min_confidence;
  const sort = normalizeSort(searchParams.sort ?? "newest");
  const limit = Math.min(100, Math.max(1, searchParams.limit ?? DEFAULT_LIMIT));
  const offset = Math.max(0, searchParams.offset ?? 0);

  const observationsQuery = useQuery({
    queryKey: ["observations", entityId, from, to, minConfidence, sort, limit, offset],
    queryFn: () => listObservations({ entityId, from, to, minConfidence, sort, limit, offset }),
  });

  const handleFilterChange = (key: keyof ObservationSearchParams, value: string | number | undefined) => {
    const newSearch = { ...searchParams, [key]: value, offset: 0 };
    (Object.keys(newSearch) as Array<keyof ObservationSearchParams>).forEach((k) => {
      if (newSearch[k] === undefined) delete newSearch[k];
    });
    void navigate({ to: "/observations", search: newSearch });
  };

  const handleSortChange = (newSort: ObservationSort) => {
    void navigate({ to: "/observations", search: { ...searchParams, sort: newSort, offset: 0 } });
  };

  const handlePageChange = (newOffset: number) => {
    void navigate({ to: "/observations", search: { ...searchParams, offset: newOffset } });
  };

  const total = observationsQuery.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / limit));
  const currentPage = Math.min(pageCount, Math.max(1, Math.floor(offset / limit) + 1));

  return (
    <section className="space-y-6">
      <div>
        <h1 className="text-3xl font-semibold tracking-tight">Observation Timeline</h1>
        <p className="mt-2 text-muted-foreground">Chronological observations with entity and decision links.</p>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Input
          placeholder="Filter by entity ID"
          className="max-w-xs"
          value={entityId ?? ""}
          onChange={(e) => handleFilterChange("entity_id", e.target.value || undefined)}
        />
        <Input
          type="date"
          placeholder="From"
          className="w-auto"
          value={from ?? ""}
          onChange={(e) => handleFilterChange("from", e.target.value || undefined)}
        />
        <Input
          type="date"
          placeholder="To"
          className="w-auto"
          value={to ?? ""}
          onChange={(e) => handleFilterChange("to", e.target.value || undefined)}
        />
        <Input
          type="number"
          min="0"
          max="1"
          step="0.1"
          placeholder="Min confidence"
          className="w-auto max-w-[100px]"
          value={minConfidence ?? ""}
          onChange={(e) => handleFilterChange("min_confidence", e.target.value ? Number(e.target.value) : undefined)}
        />
      </div>

      <div className="flex items-center gap-2">
        <span className="text-sm font-medium">Sort by:</span>
        {SORT_OPTIONS.map((s) => (
          <Button
            key={s}
            type="button"
            variant={sort === s ? "default" : "outline"}
            size="sm"
            onClick={() => handleSortChange(s)}
          >
            {s.charAt(0).toUpperCase() + s.slice(1)}
          </Button>
        ))}
      </div>

      {observationsQuery.isLoading ? (
        <ScrollArea className="h-[400px]">
          <div className="space-y-4">
            <Skeleton className="h-24 w-full" />
            <Skeleton className="h-24 w-full" />
            <Skeleton className="h-24 w-full" />
          </div>
        </ScrollArea>
      ) : observationsQuery.isError ? (
        <p className="text-sm text-destructive">Failed to load observations. Ensure the API is running.</p>
      ) : observationsQuery.data?.observations.length === 0 ? (
        <Card>
          <CardHeader>
            <CardTitle>No observations found</CardTitle>
            <CardDescription>Try adjusting your filters or date range.</CardDescription>
          </CardHeader>
        </Card>
      ) : (
        <>
          <p className="text-sm text-muted-foreground">Total: {total}</p>
          <ScrollArea className="h-[500px]">
            <div className="relative pl-6 border-l space-y-4">
              {observationsQuery.data?.observations.map((observation, index) => (
                <ObservationCard
                  key={observation.id}
                  observation={observation}
                  index={index}
                  total={observationsQuery.data!.observations.length}
                />
              ))}
            </div>
          </ScrollArea>
          <div className="flex items-center justify-between gap-2">
            <p className="text-sm text-muted-foreground">
              Showing {total > 0 ? offset + 1 : 0}-{Math.min(offset + limit, total)} of {total}
            </p>
            <div className="flex items-center gap-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={currentPage <= 1}
                onClick={() => handlePageChange((currentPage - 2) * limit)}
              >
                Previous
              </Button>
              <p className="text-sm text-muted-foreground">
                Page {currentPage} of {pageCount}
              </p>
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={currentPage >= pageCount}
                onClick={() => handlePageChange(currentPage * limit)}
              >
                Next
              </Button>
            </div>
          </div>
        </>
      )}
    </section>
  );
}