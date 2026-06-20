import { z } from "zod";
import { getJson } from "./client";

export const entitySchema = z.object({
  uri: z.string(),
  label: z.string(),
});
export type Entity = z.infer<typeof entitySchema>;

export const entitySummarySchema = z.object({
  uri: z.string(),
  label: z.string(),
  types: z.array(z.string()),
  decision_count: z.number().optional(),
  observation_count: z.number().optional(),
});
export type EntitySummary = z.infer<typeof entitySummarySchema>;

export const entitiesResponseSchema = z.object({
  entities: z.array(entitySummarySchema),
  count: z.number(),
});
export type EntitiesResponse = z.infer<typeof entitiesResponseSchema>;

export const observationRefSchema = z.object({
  uri: z.string(),
  text: z.string(),
});
export type ObservationRef = z.infer<typeof observationRefSchema>;

export const sourceLocationSchema = z.object({
  uri: z.string(),
  label: z.string(),
});
export type SourceLocation = z.infer<typeof sourceLocationSchema>;

const entityRefSchema = z.object({ uri: z.string(), label: z.string() });

const relationSchema = z.object({
  subject_uri: z.string(),
  predicate_uri: z.string(),
  object_uri: z.string(),
  label: z.string(),
});

export const entityDetailSchema = z.object({
  uri: z.string(),
  label: z.string(),
  types: z.array(z.string()),
  related_decisions: z.array(entityRefSchema),
  related_observations: z.array(observationRefSchema),
  relationships: z.array(relationSchema),
  source_locations: z.array(sourceLocationSchema),
});
export type EntityDetail = z.infer<typeof entityDetailSchema>;

export const decisionRefSchema = z.object({
  id: z.string(),
  title: z.string(),
  status: z.string(),
});
export type DecisionRef = z.infer<typeof decisionRefSchema>;

export const entityDecisionsResponseSchema = z.object({
  decisions: z.array(decisionRefSchema),
  count: z.number(),
});
export type EntityDecisionsResponse = z.infer<typeof entityDecisionsResponseSchema>;

export const entityObservationsResponseSchema = z.object({
  observations: z.array(observationRefSchema),
  count: z.number(),
});
export type EntityObservationsResponse = z.infer<typeof entityObservationsResponseSchema>;

export function listEntities(q?: string, limit?: number) {
  const params = new URLSearchParams();
  if (q) params.set("q", q);
  if (limit !== undefined) params.set("limit", String(limit));
  const qs = params.toString();
  return getJson<EntitiesResponse>(`/api/v1/entities${qs ? "?" + qs : ""}`).then((v) =>
    entitiesResponseSchema.parse(v),
  );
}

export function getEntity(id: string) {
  return getJson<EntityDetail>(`/api/v1/entities/${encodeURIComponent(id)}`).then((v) =>
    entityDetailSchema.parse(v),
  );
}

export function getEntityDecisions(id: string) {
  return getJson<EntityDecisionsResponse>(`/api/v1/entities/${encodeURIComponent(id)}/decisions`).then((v) =>
    entityDecisionsResponseSchema.parse(v),
  );
}

export function getEntityObservations(id: string) {
  return getJson<EntityObservationsResponse>(`/api/v1/entities/${encodeURIComponent(id)}/observations`).then((v) =>
    entityObservationsResponseSchema.parse(v),
  );
}
