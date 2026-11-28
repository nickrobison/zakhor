import { Outlet, createFileRoute } from "@tanstack/react-router";
import { z } from "zod";

// Stale links from older builds can still carry removed values (`proposed`,
// `archived`, `referenced`, `confidence`). Fall back to the page defaults
// instead of failing navigation with a ZodError.
const searchParamsSchema = z.object({
  q: z.string().optional(),
  status: z.enum(["active", "superseded"]).optional().catch(undefined),
  sort: z.enum(["created", "modified"]).optional().catch(undefined),
  limit: z.coerce.number().int().min(1).max(100).optional(),
  offset: z.coerce.number().int().min(0).optional(),
});

export const Route = createFileRoute("/decisions")({
  validateSearch: (search: Record<string, unknown>) => searchParamsSchema.parse(search),
  component: DecisionsLayout,
});

function DecisionsLayout() {
  return <Outlet />;
}