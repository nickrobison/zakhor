import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Link } from "@tanstack/react-router";
import { Badge } from "@/components/ui/badge";
import { EmptyState } from "@/components/shared/EmptyState";
import { SearchInput } from "@/components/shared/SearchInput";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { listEntities } from "@/lib/api/entities";

const DEFAULT_LIMIT = 50;

export function EntitiesPage() {
  const navigate = useNavigate();
  const searchParams = useSearch({ from: "/entities" });

  const q = searchParams.q ?? "";
  const limit = Math.min(100, Math.max(1, searchParams.limit ?? DEFAULT_LIMIT));

  const entitiesQuery = useQuery({
    queryKey: ["entities", q, limit],
    queryFn: () => listEntities(q, limit),
  });

  const handleInputChange = (value: string) => {
    void navigate({ to: "/entities", search: { q: value, limit } });
  };

  const total = entitiesQuery.data?.count ?? 0;

  return (
    <section className="space-y-6">
      <div>
        <h1 className="text-3xl font-semibold tracking-tight">Entity Explorer</h1>
        <p className="mt-2 text-muted-foreground">Entities, relationships, decisions, and observations.</p>
      </div>

      <form onSubmit={(e) => e.preventDefault()}>
        <SearchInput
          className="max-w-sm"
          placeholder="Search entities by label"
          value={q}
          onChange={handleInputChange}
        />
      </form>

      {entitiesQuery.isLoading ? (
        <div className="space-y-3">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : entitiesQuery.isError ? (
        <p className="text-sm text-destructive">Failed to load entities. Ensure the API is running.</p>
      ) : entitiesQuery.data?.entities.length === 0 ? (
        <EmptyState title="No entities found" description="Try adjusting your search terms." />
      ) : (
        <>
          <p className="text-sm text-muted-foreground">Total: {total}</p>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Label</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Decisions</TableHead>
                <TableHead>Observations</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {entitiesQuery.data?.entities.map((entity) => (
                <TableRow key={entity.uri}>
                  <TableCell>
                    <Link
                      to="/entities/$entityId"
                      params={{ entityId: entity.uri }}
                      className="font-medium hover:underline"
                    >
                      {entity.label}
                    </Link>
                  </TableCell>
                  <TableCell>
                    {entity.types.length > 0 ? (
                      entity.types.map((type) => (
                        <Badge key={type} variant="secondary" className="mr-1">
                          {type}
                        </Badge>
                      ))
                    ) : (
                      <span className="text-muted-foreground text-sm">—</span>
                    )}
                  </TableCell>
                  <TableCell>{entity.decision_count ?? 0}</TableCell>
                  <TableCell>{entity.observation_count ?? 0}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </>
      )}
    </section>
  );
}
