import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { getAdminStatus, rebuildIndexes, type AdminStatus } from "@/lib/api/admin";
import { getHealth } from "@/lib/api/health";

const REFRESH_INTERVAL_MS = 30_000;

export function AdminPage() {
  const queryClient = useQueryClient();

  const health = useQuery({ queryKey: ["admin-health"], queryFn: getHealth, retry: false });
  const status = useQuery({
    queryKey: ["admin-status"],
    queryFn: getAdminStatus,
    retry: false,
    refetchInterval: REFRESH_INTERVAL_MS,
  });
  const rebuild = useMutation({
    mutationFn: rebuildIndexes,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["admin-status"] });
    },
  });

  function handleRebuild() {
    const confirmed = window.confirm("Rebuild lexical and semantic indexes? This may take some time.");
    if (!confirmed) return;
    rebuild.mutate();
  }

  return (
    <section className="space-y-6">
      <div>
        <h1 className="text-3xl font-semibold tracking-tight">Administration Console</h1>
        <p className="mt-2 text-muted-foreground">Tracker status, index health, and rebuild controls.</p>
      </div>

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <StatusCard title="Tracker" value={health.isLoading ? "Loading" : health.data?.status ?? "Unknown"} variant={healthStatusVariant(health.data?.status)} />
        <StatusCard title="Indexes" value={status.isLoading ? "Loading" : status.data?.indexes_available ? "Available" : "Unavailable"} variant={status.data?.indexes_available ? "default" : "secondary"} />
        <StatusCard title="Lexical docs" value={status.isLoading ? "Loading" : status.data?.lexical_docs.toLocaleString() ?? "0"} variant="secondary" />
        <StatusCard title="Semantic vectors" value={status.isLoading ? "Loading" : status.data?.semantic_vectors.toLocaleString() ?? "0"} variant="secondary" />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Index status</CardTitle>
          <CardDescription>Refreshes automatically every 30 seconds.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {status.isLoading ? (
            <AdminStatusSkeleton />
          ) : status.isError ? (
            <p className="text-sm text-destructive">Failed to load admin status. Ensure the Rust API is running.</p>
          ) : status.data ? (
            <StatusDetails status={status.data} />
          ) : null}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Rebuild indexes</CardTitle>
          <CardDescription>POST /api/v1/admin/rebuild-indexes to trigger an asynchronous rebuild.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <Button type="button" variant="outline" disabled={(status.data?.rebuild_in_progress ?? false) || rebuild.isPending} onClick={handleRebuild}>
            {rebuild.isPending ? "Rebuilding…" : "Rebuild indexes"}
          </Button>
          {status.data?.rebuild_in_progress ? <p className="text-sm text-muted-foreground">A rebuild is already in progress.</p> : null}
          {rebuild.isError ? <p className="text-sm text-destructive">Rebuild request failed. Try again after the API is available.</p> : null}
          {rebuild.isSuccess ? <p className="text-sm text-muted-foreground">Rebuild accepted. Status will refresh automatically.</p> : null}
        </CardContent>
      </Card>
    </section>
  );
}

function StatusDetails({ status }: { status: AdminStatus }) {
  return (
    <div className="space-y-4 text-sm">
      <div className="grid gap-3 md:grid-cols-2">
        <StatusMetric label="Lexical documents" value={status.lexical_docs.toLocaleString()} />
        <StatusMetric label="Semantic vectors" value={status.semantic_vectors.toLocaleString()} />
        <StatusMetric label="Rebuild state" value={status.rebuild_in_progress ? "In progress" : "Idle"} />
        <StatusMetric label="Last rebuild" value={formatLastRebuild(status.last_rebuild_at_ms)} />
      </div>
      <Separator />
      <p className="text-muted-foreground">Indexes available: {status.indexes_available ? "yes" : "no"}</p>
    </div>
  );
}

function StatusMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border p-3">
      <p className="text-muted-foreground">{label}</p>
      <p className="mt-1 font-medium text-foreground">{value}</p>
    </div>
  );
}

function StatusCard({ title, value, variant }: { title: string; value: string; variant: "default" | "secondary" | "outline" }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <Badge variant={variant}>{value}</Badge>
      </CardContent>
    </Card>
  );
}

function AdminStatusSkeleton() {
  return (
    <div className="space-y-3">
      <Skeleton className="h-16 w-full" />
      <Skeleton className="h-16 w-full" />
      <Skeleton className="h-16 w-full" />
    </div>
  );
}

function healthStatusVariant(status: string | undefined): "default" | "secondary" | "outline" {
  if (status === "ok") return "default";
  if (status === "tracker_unavailable") return "secondary";
  return "outline";
}

function formatLastRebuild(value: number | null | undefined) {
  if (!value) return "Not run";
  return new Date(value).toLocaleString();
}
