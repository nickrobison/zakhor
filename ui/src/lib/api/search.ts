import { z } from "zod";
import { getJson } from "./client";

export const searchModeSchema = z.enum(["hybrid", "lexical", "semantic"]);

export type SearchMode = z.infer<typeof searchModeSchema>;

export const searchResultSchema = z.object({
  id: z.string(),
  score: z.number(),
});

export const searchResponseSchema = z.object({
  results: z.array(searchResultSchema),
  count: z.number(),
  warning: z.string().nullable().optional(),
});

export type SearchResponse = z.infer<typeof searchResponseSchema>;

export function searchHybrid(query: string, limit = 20, mode: SearchMode = "hybrid") {
  const params = new URLSearchParams({ q: query, limit: String(limit), mode });
  return getJson<SearchResponse>(`/api/v1/search?${params}`).then((value) => searchResponseSchema.parse(value));
}
