import { Link } from "@tanstack/react-router";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

export function EntityTag({ uri, label = uri, className }: { uri: string; label?: string; className?: string }) {
  return (
    <Link to="/entities/$entityId" params={{ entityId: uri }}>
      <Badge className={cn("cursor-pointer", className)} variant="secondary">
        {label}
      </Badge>
    </Link>
  );
}
