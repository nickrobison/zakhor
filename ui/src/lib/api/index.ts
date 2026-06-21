import { getHealth } from "@/lib/api/health";

export async function fetchHealth() {
  return getHealth();
}
