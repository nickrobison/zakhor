import type * as React from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Link } from "@tanstack/react-router";
import { SearchInput } from "@/components/shared/SearchInput";
import { StatusBadge } from "@/components/shared/StatusBadge";
import { ConfidenceIndicator } from "@/components/shared/ConfidenceIndicator";
import { EntityTag } from "@/components/shared/EntityTag";
import { PaginationControls } from "@/components/shared/PaginationControls";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { listDecisions, type DecisionStatus, type DecisionSort, type DecisionSummary } from "@/lib/api/decisions";

const DEFAULT_LIMIT = 20;
const STATUS_OPTIONS: DecisionStatus[] = ["active", "proposed", "superseded", "archived"];
const SORT_OPTIONS: DecisionSort[] = ["modified", "created", "referenced", "confidence"];

function normalizeStatus(value: string): DecisionStatus {
  if (STATUS_OPTIONS.includes(value as DecisionStatus)) return value as DecisionStatus;
  return "active";
}

function normalizeSort(value: string): DecisionSort {
  if (SORT_OPTIONS.includes(value as DecisionSort)) return value as DecisionSort;
  return "modified";
}

export function DecisionsPage() {
  const navigate = useNavigate();
  const searchParams = useSearch({ from: "/decisions" });

  const q = searchParams.q ?? "";
  const status = normalizeStatus(searchParams.status ?? "active");
  const sort = normalizeSort(searchParams.sort ?? "modified");
  const limit = Math.min(100, Math.max(1, searchParams.limit ?? DEFAULT_LIMIT));
  const offset = Math.max(0, searchParams.offset ?? 0);

  const decisionsQuery = useQuery({
    queryKey: ["decisions", q, status, sort, limit, offset],
    queryFn: () => listDecisions({ q, status, sort, limit, offset }),
  });

  const handleSearchSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    void navigate({ to: "/decisions", search: { q, status, sort, limit, offset: 0 } });
  };

  const handleFilterChange = (newStatus: DecisionStatus) => {
    void navigate({ to: "/decisions", search: { q, status: newStatus, sort, limit, offset: 0 } });
  };

  const handleSortChange = (newSort: DecisionSort) => {
    void navigate({ to: "/decisions", search: { q, status, sort: newSort, limit, offset: 0 } });
  };

  const handleInputChange = (value: string) => {
    void navigate({ to: "/decisions", search: { q: value, status, sort, limit, offset: 0 } });
  };

  const total = decisionsQuery.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / limit));
  const currentPage = Math.min(pageCount, Math.max(1, Math.floor(offset / limit) + 1));

  const handlePageChange = (newPage: number) => {
    const nextOffset = (newPage - 1) * limit;
    void navigate({ to: "/decisions", search: { q, status, sort, limit, offset: nextOffset } });
  };

  return (
    <section className="space-y-6">
      <div>
        <h1 className="text-3xl font-semibold tracking-tight">Decision Explorer</h1>
        <p className="mt-2 text-muted-foreground">List and detail views with provenance panel.</p>
      </div>

      <form className="flex items-center gap-2" onSubmit={handleSearchSubmit}>
        <SearchInput
          className="max-w-sm"
          placeholder="Search decisions"
          value={q}
          onChange={handleInputChange}
        />
        {STATUS_OPTIONS.map((s) => (
          <Button
            key={s}
            type="button"
            variant={status === s ? "default" : "outline"}
            size="sm"
            onClick={() => handleFilterChange(s)}
          >
            {s.charAt(0).toUpperCase() + s.slice(1)}
          </Button>
        ))}
      </form>

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

      {decisionsQuery.isLoading ? (
        <div className="space-y-3">
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-20 w-full" />
        </div>
      ) : decisionsQuery.isError ? (
        <p className="text-sm text-destructive">Failed to load decisions. Ensure the API is running.</p>
      ) : decisionsQuery.data?.decisions.length === 0 ? (
        <Card>
          <CardHeader>
            <CardTitle>No decisions found</CardTitle>
            <CardDescription>Try adjusting your search, status filter, or sort order.</CardDescription>
          </CardHeader>
        </Card>
      ) : (
        <div className="grid gap-4">
          {decisionsQuery.data?.decisions.map((decision) => (
            <DecisionCard key={decision.id} decision={decision} />
          ))}
          <PaginationControls currentPage={currentPage} pageCount={pageCount} onPageChange={handlePageChange} pageSize={limit} total={total} />
        </div>
      )}
    </section>
  );
}

function DecisionCard({ decision }: { decision: DecisionSummary & { confidence?: number | null; evidence_count?: number | null; entity_tags?: { uri: string; label: string }[] | null } }) {
  return (
    <Card className="transition-colors hover:bg-accent">
      <CardHeader>
        <div className="flex items-center justify-between gap-4">
          <Link
            to="/decisions/$decisionId"
            params={{ decisionId: decision.id }}
            className="flex-1 hover:underline"
          >
            <CardTitle>{decision.title}</CardTitle>
          </Link>
          <StatusBadge status={decision.status as "active" | "proposed" | "superseded" | "archived" | "ok" | "warning" | "error" | "unknown"} />
        </div>
        <CardDescription className="flex items-center gap-4 mt-2">
          {decision.modified ? (
            <span>Modified: {new Date(decision.modified).toLocaleDateString()}</span>
          ) : null}
          {decision.confidence != null && <ConfidenceIndicator value={decision.confidence} label="Confidence" />}
          {decision.evidence_count != null && (
            <span className="text-sm">{decision.evidence_count} evidence</span>
          )}
        </CardDescription>
        {decision.entity_tags && decision.entity_tags.length > 0 ? (
          <div className="flex flex-wrap gap-1 mt-2">
            {decision.entity_tags.map((tag) => (
              <EntityTag key={tag.uri} uri={tag.uri} label={tag.label} />
            ))}
          </div>
        ) : null}
      </CardHeader>
    </Card>
  );
}
