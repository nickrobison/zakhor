import { z } from "zod";
import { getJson } from "./client";

export type ObservationSort = "newest" | "oldest";

export const observationSummarySchema = z.object({
  id: z.string(),
  text: z.string(),
  created_at: z.string().nullable().optional(),
  confidence: z.number().nullable().optional(),
  entity_refs: z.array(z.string()).optional(),
  source: z.string().nullable().optional(),
  decision_refs: z.array(z.string()).optional(),
});

export const observationListResponseSchema = z.object({
  observations: z.array(observationSummarySchema),
  count: z.number(),
  total: z.number(),
});

export const observationDetailSchema = z.object({
  id: z.string(),
  content: z.string(),
  created_at: z.string().nullable().optional(),
  confidence: z.number().nullable().optional(),
  entity_refs: z.array(z.string()),
  source: z.string().nullable().optional(),
  decision_refs: z.array(z.string()),
});

export type ObservationSummary = z.infer<typeof observationSummarySchema>;
export type ObservationListResponse = z.infer<typeof observationListResponseSchema>;
export type ObservationDetail = z.infer<typeof observationDetailSchema>;

export function listObservations(options: {
  offset?: number;
  limit?: number;
  entityId?: string;
  from?: string;
  to?: string;
  minConfidence?: number;
  sort?: ObservationSort;
} = {}) {
  const params = new URLSearchParams();
  if (options.offset !== undefined) params.set("offset", String(options.offset));
  if (options.limit !== undefined) params.set("limit", String(options.limit));
  if (options.entityId) params.set("entity_id", options.entityId);
  if (options.from) params.set("from", options.from);
  if (options.to) params.set("to", options.to);
  if (options.minConfidence !== undefined) params.set("min_confidence", String(options.minConfidence));
  if (options.sort) params.set("sort", options.sort);
  const qs = params.toString();
  return getJson<ObservationListResponse>(`/api/v1/observations${qs ? "?" + qs : ""}`).then((value) =>
    observationListResponseSchema.parse(value),
  );
}

export function getObservation(id: string) {
  return getJson<ObservationDetail>(`/api/v1/observations/${encodeURIComponent(id)}`).then((value) =>
    observationDetailSchema.parse(value),
  );
}