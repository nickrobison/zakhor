import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { ObservationsPage } from "@/pages/ObservationsPage";

const searchParamsSchema = z.object({
  entity_id: z.string().optional(),
  from: z.string().optional(),
  to: z.string().optional(),
  min_confidence: z.coerce.number().min(0).max(1).optional(),
  sort: z.enum(["newest", "oldest"]).optional(),
  offset: z.coerce.number().int().min(0).optional(),
  limit: z.coerce.number().int().min(1).max(100).optional(),
});

export const Route = createFileRoute("/observations")({
  validateSearch: (search: Record<string, unknown>) => searchParamsSchema.parse(search),
  component: ObservationsPage,
});