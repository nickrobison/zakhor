import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

export type StatusKind = "active" | "proposed" | "superseded" | "archived" | "ok" | "warning" | "error" | "unknown";

const variantByStatus: Record<StatusKind, "default" | "secondary" | "outline" | "destructive"> = {
  active: "default",
  proposed: "secondary",
  superseded: "outline",
  archived: "outline",
  ok: "default",
  warning: "secondary",
  error: "destructive",
  unknown: "outline",
};

export function StatusBadge({ status, className }: { status: StatusKind; className?: string }) {
  return <Badge className={cn("capitalize", className)} variant={variantByStatus[status]}>{status}</Badge>;
}
