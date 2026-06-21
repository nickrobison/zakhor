import { Outlet, createFileRoute } from "@tanstack/react-router";
import { z } from "zod";

const searchParamsSchema = z.object({
  q: z.string().optional(),
  limit: z.coerce.number().int().min(1).max(100).optional(),
});

export const Route = createFileRoute("/entities")({
  validateSearch: (search: Record<string, unknown>) => searchParamsSchema.parse(search),
  component: EntitiesLayout,
});

function EntitiesLayout() {
  return <Outlet />;
}
