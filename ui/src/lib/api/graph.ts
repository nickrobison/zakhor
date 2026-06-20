import { z } from "zod";
import { getJson } from "./client";

export const graphResponseSchema = z.object({
  triples: z.array(z.object({ subject: z.string(), predicate: z.string(), object: z.string() })),
  count: z.number(),
  warning: z.string().nullable().optional(),
});

export type GraphResponse = z.infer<typeof graphResponseSchema>;

export function traverseGraph(startId: string, depth = 1, edgeTypes: string[] = []) {
  const params = new URLSearchParams({ start_id: startId, depth: String(depth) });
  for (const edgeType of edgeTypes) {
    params.append("edge_types", edgeType);
  }
  return getJson<GraphResponse>(`/api/v1/graph/traverse?${params}`).then((value) => graphResponseSchema.parse(value));
}
