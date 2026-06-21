import { z } from "zod";

export const healthSchema = z.object({
  status: z.enum(["ok", "tracker_unavailable"]),
});

export type Health = z.infer<typeof healthSchema>;
