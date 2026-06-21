import { z } from "zod";
import { getJson, postJson } from "./client";

export const adminStatusSchema = z.object({
  rebuild_in_progress: z.boolean(),
  lexical_docs: z.number(),
  semantic_vectors: z.number(),
  last_rebuild_at_ms: z.number().nullable().optional(),
  indexes_available: z.boolean(),
});

export const rebuildResponseSchema = z.object({
  status: z.string(),
  message: z.string(),
});

export type AdminStatus = z.infer<typeof adminStatusSchema>;
export type RebuildResponse = z.infer<typeof rebuildResponseSchema>;

export function getAdminStatus() {
  return getJson<AdminStatus>("/api/v1/admin/status").then((value) => adminStatusSchema.parse(value));
}

export function rebuildIndexes() {
  return postJson<RebuildResponse>("/api/v1/admin/rebuild-indexes");
}
