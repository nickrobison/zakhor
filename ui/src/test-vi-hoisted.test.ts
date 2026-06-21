import { vi, it, expect } from "vitest";
it("vi.hoisted exists", () => {
  expect(typeof vi.hoisted).toBe("function");
});
