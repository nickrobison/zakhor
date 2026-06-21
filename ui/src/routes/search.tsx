import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { SearchPage } from "@/pages/SearchPage";

const searchParamsSchema = z.object({
  q: z.string().optional(),
  limit: z.coerce.number().int().min(1).max(100).optional(),
  mode: z.enum(["hybrid", "lexical", "semantic"]).optional(),
  type: z.enum(["all", "decisions", "entities", "observations"]).optional(),
  page: z.coerce.number().int().min(1).optional(),
});

export const Route = createFileRoute("/search")({
  validateSearch: (search: Record<string, unknown>) => searchParamsSchema.parse(search),
  component: SearchPage,
});

export type SearchRouteParams = z.infer<typeof searchParamsSchema>;
