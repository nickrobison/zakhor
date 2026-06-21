import { getJson } from "./client";
import { healthSchema, type Health } from "./schemas";

export function getHealth() {
  return getJson<Health>("/api/v1/health").then((value) => healthSchema.parse(value));
}
