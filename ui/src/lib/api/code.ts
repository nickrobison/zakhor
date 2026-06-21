import { z } from "zod";
import { getJson } from "./client";

const codeRepositorySchema = z.object({
  name: z.string(),
  url: z.string().nullable().optional(),
  description: z.string().nullable().optional(),
});

const codeFileSchema = z.object({
  path: z.string(),
  repository: z.string().nullable().optional(),
  language: z.string().nullable().optional(),
});

const codeSymbolSchema = z.object({
  name: z.string(),
  kind: z.string(),
  file_path: z.string(),
  line: z.number(),
});

export const codeResponseSchema = z.object({
  files: z.array(codeFileSchema),
  symbols: z.array(codeSymbolSchema),
  repositories: z.array(codeRepositorySchema),
});

export type CodeResponse = z.infer<typeof codeResponseSchema>;

export function getCodeReferences(q?: string, repo?: string) {
  const params = new URLSearchParams();
  if (q) params.set("q", q);
  if (repo) params.set("repo", repo);
  const query = params.toString();
  return getJson<CodeResponse>(`/api/v1/code${query ? `?${query}` : ""}`).then((value) =>
    codeResponseSchema.parse(value),
  );
}
