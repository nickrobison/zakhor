import { z } from "zod";
import { getJson } from "./client";

export type DecisionStatus = "active" | "superseded" | "proposed" | "archived";
export type DecisionSort = "modified" | "created" | "referenced" | "confidence";

export const decisionSummarySchema = z.object({
  id: z.string(),
  title: z.string(),
  status: z.string(),
  created: z.string().nullable().optional(),
  modified: z.string().nullable().optional(),
  confidence: z.number().nullable().optional(),
  evidence_count: z.number().nullable().optional(),
  entity_tags: z.array(z.object({ uri: z.string(), label: z.string() })).nullable().optional(),
});
export type DecisionSummary = z.infer<typeof decisionSummarySchema>;

export const decisionsResponseSchema = z.object({
  decisions: z.array(decisionSummarySchema),
  count: z.number(),
  total: z.number(),
});
export type DecisionsResponse = z.infer<typeof decisionsResponseSchema>;

export const evidenceItemSchema = z.object({
  source: z.string(),
  content: z.string(),
});
export type EvidenceItem = z.infer<typeof evidenceItemSchema>;

export const codeReferenceSchema = z.object({
  file_path: z.string(),
  repo: z.string().nullable().optional(),
});
export type CodeReference = z.infer<typeof codeReferenceSchema>;

export const decisionDetailSchema = z.object({
  id: z.string(),
  title: z.string(),
  status: z.string(),
  created: z.string().nullable().optional(),
  modified: z.string().nullable().optional(),
  confidence: z.number().nullable().optional(),
  summary: z.string().nullable().optional(),
  context: z.string(),
  outcome: z.string(),
  rationale: z.string(),
  alternatives: z.array(z.string()),
  evidence: z.array(evidenceItemSchema),
  entities: z.array(z.object({ uri: z.string(), label: z.string() })),
  related_decision_ids: z.array(z.string()),
  code_references: z.array(codeReferenceSchema).nullable().optional(),
});
export type DecisionDetail = z.infer<typeof decisionDetailSchema>;

export const provenanceItemSchema = z.object({
  step: z.string(),
  label: z.string(),
  source: z.string(),
});
export type ProvenanceItem = z.infer<typeof provenanceItemSchema>;

export const provenanceResponseSchema = z.object({
  chain: z.array(provenanceItemSchema),
  count: z.number(),
});
export type ProvenanceResponse = z.infer<typeof provenanceResponseSchema>;

export function listDecisions(options: {
  q?: string;
  status?: DecisionStatus;
  sort?: DecisionSort;
  limit?: number;
  offset?: number;
} = {}) {
  const params = new URLSearchParams();
  if (options.q) params.set("q", options.q);
  if (options.status) params.set("status", options.status);
  if (options.sort) params.set("sort", options.sort);
  if (options.limit !== undefined) params.set("limit", String(options.limit));
  if (options.offset !== undefined) params.set("offset", String(options.offset));
  const qs = params.toString();
  return getJson<DecisionsResponse>(`/api/v1/decisions${qs ? "?" + qs : ""}`).then((v) =>
    decisionsResponseSchema.parse(v),
  );
}

export function getDecision(id: string) {
  return getJson<DecisionDetail>(`/api/v1/decisions/${encodeURIComponent(id)}`).then((v) =>
    decisionDetailSchema.parse(v),
  );
}

export function getDecisionProvenance(id: string) {
  return getJson<ProvenanceResponse>(`/api/v1/decisions/${encodeURIComponent(id)}/provenance`).then((v) =>
    provenanceResponseSchema.parse(v),
  );
}
