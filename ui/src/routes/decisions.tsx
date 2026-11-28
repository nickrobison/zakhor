import { Outlet, createFileRoute } from "@tanstack/react-router";
import { z } from "zod";

const searchParamsSchema = z.object({
  q: z.string().optional(),
  status: z.enum(["active", "superseded"]).optional(),
  sort: z.enum(["created", "modified"]).optional(),
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