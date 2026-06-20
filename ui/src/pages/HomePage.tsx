import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { fetchHealth } from "@/lib/api";
import { getAdminStatus } from "@/lib/api/admin";
import { listDecisions } from "@/lib/api/decisions";
import { listEntities } from "@/lib/api/entities";
import { listObservations } from "@/lib/api/observations";

const navStats = [
  {
    label: "Decisions",
    href: "/decisions",
    description: "Lifecycle and rationale index",
    valueKey: "decisions",
  },
  {
    label: "Entities",
    href: "/entities",
    description: "Technologies, components, projects, issues",
    valueKey: "entities",
  },
  {
    label: "Observations",
    href: "/observations",
    description: "Chronological memory timeline",
    valueKey: "observations",
  },
  {
    label: "Graph",
    href: "/graph",
    description: "Depth-limited relationship explorer",
    valueKey: "graph",
  },
] as const;

type NavStatValue = (typeof navStats)[number]["valueKey"];

export function HomePage() {
  const navigate = useNavigate();
  const [quickQuery, setQuickQuery] = useState("");

  const health = useQuery({ queryKey: ["health"], queryFn: fetchHealth, retry: false });
  const status = useQuery({ queryKey: ["admin-status"], queryFn: getAdminStatus, retry: false });
  const decisions = useQuery({
    queryKey: ["decisions", "home"],
    queryFn: () => listDecisions({ limit: 5, sort: "modified" }),
    retry: false,
  });
  const entities = useQuery({
    queryKey: ["entities", "home"],
    queryFn: () => listEntities(undefined, 5),
    retry: false,
  });
  const observations = useQuery({
    queryKey: ["observations", "home"],
    queryFn: () => listObservations({ offset: 0, limit: 5 }),
    retry: false,
  });

  const indexState = useMemo(() => {
    if (status.isLoading) return { label: "Indexing", variant: "secondary" } as const;
    if (status.isError) return { label: "Unavailable", variant: "destructive" } as const;
    if (!status.data?.indexes_available) return { label: "Pending", variant: "secondary" } as const;
    return { label: "Ready", variant: "default" } as const;
  }, [status.data?.indexes_available, status.isError, status.isLoading]);

  const statValues = useMemo<Record<NavStatValue, string | number>>(() => {
    const decisionCount = decisions.data?.total ?? "—";
    const entityCount = entities.data?.count ?? "—";
    const observationCount = observations.data?.total ?? "—";
    const lexicalDocs = status.data?.lexical_docs.toLocaleString() ?? "—";

    return {
      decisions: decisionCount,
      entities: entityCount,
      observations: observationCount,
      graph: lexicalDocs,
    };
  }, [
    decisions.data?.total,
    entities.data?.count,
    observations.data?.total,
    status.data?.lexical_docs,
  ]);

  const recentActivity = useMemo(
    () =>
      [
        ...observations.data?.observations.map((observation) => ({
          kind: "Observation" as const,
          title: observation.text,
          meta: observation.created_at ?? "No timestamp",
        })) ?? [],
        ...decisions.data?.decisions.map((decision) => ({
          kind: "Decision" as const,
          title: decision.title,
          meta: `${decision.status} · ${decision.modified ?? decision.created ?? "No timestamp"}`,
        })) ?? [],
      ].slice(0, 5),
    [decisions.data?.decisions, observations.data?.observations],
  );

  function submitQuickSearch(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = quickQuery.trim();
    if (!trimmed) return;

    void navigate({ to: "/search", search: { q: trimmed, mode: "hybrid", type: "all" } });
  }

  return (
    <section className="space-y-8">
      <div className="space-y-3">
        <div>
          <h1 className="text-3xl font-semibold tracking-tight">Zakhor Web UI</h1>
          <p className="mt-2 text-muted-foreground">Operational dashboard for the knowledge graph memory system.</p>
        </div>

        <form className="flex flex-col gap-2 sm:flex-row" onSubmit={submitQuickSearch}>
          <label className="sr-only" htmlFor="quick-search">
            Quick search
          </label>
          <Input
            id="quick-search"
            value={quickQuery}
            onChange={(event) => setQuickQuery(event.target.value)}
            placeholder="Search decisions, entities, and observations"
            className="sm:max-w-md"
          />
          <Button type="submit" disabled={!quickQuery.trim()}>
            Search
          </Button>
        </form>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-start justify-between gap-4">
            <div>
              <CardTitle>System status</CardTitle>
              <CardDescription>Tracker health and derived index readiness</CardDescription>
            </div>
            <Badge variant={indexState.variant}>{indexState.label}</Badge>
          </div>
        </CardHeader>
        <CardContent>
          {health.isLoading ? (
            <Skeleton className="h-6 w-32" />
          ) : health.isError ? (
            <p className="text-sm text-destructive">API unavailable</p>
          ) : (
            <p className="text-sm font-medium">{health.data?.status}</p>
          )}
        </CardContent>
      </Card>

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {navStats.map((stat) => (
          <Link key={stat.href} to={stat.href}>
            <Card className="h-full transition-colors hover:bg-accent">
              <CardHeader>
                <CardTitle>{stat.label}</CardTitle>
                <CardDescription>{stat.description}</CardDescription>
              </CardHeader>
              <CardContent>
                {statValues[stat.valueKey] === "—" ? (
                  <Skeleton className="h-7 w-20" />
                ) : (
                  <p className="text-2xl font-semibold">{statValues[stat.valueKey]}</p>
                )}
              </CardContent>
            </Card>
          </Link>
        ))}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Recent activity</CardTitle>
          <CardDescription>Latest observations and modified decisions</CardDescription>
        </CardHeader>
        <CardContent>
          {observations.isLoading || decisions.isLoading ? (
            <div className="space-y-3">
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-4 w-5/6" />
              <Skeleton className="h-4 w-4/6" />
            </div>
          ) : observations.isError || decisions.isError ? (
            <p className="text-sm text-destructive">Recent activity is unavailable until the API responds.</p>
          ) : recentActivity.length === 0 ? (
            <p className="text-sm text-muted-foreground">No recent activity recorded yet.</p>
          ) : (
            <ol className="space-y-3">
              {recentActivity.map((item, index) => (
                <li key={`${item.kind}-${item.title}-${index}`} className="rounded-md border p-3">
                  <div className="flex items-center justify-between gap-3">
                    <p className="text-sm font-medium">{item.title}</p>
                    <Badge variant="outline">{item.kind}</Badge>
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">{item.meta}</p>
                </li>
              ))}
            </ol>
          )}
        </CardContent>
      </Card>
    </section>
  );
}
