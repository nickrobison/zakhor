import { vi, it, expect } from "vitest";

const mocks = vi.hoisted(() => ({ readFileSync: vi.fn() }));
vi.mock("fs", () => ({ readFileSync: mocks.readFileSync }));
import { readFileSync } from "fs";

it("mock works", () => {
  readFileSync("/test");
  expect(mocks.readFileSync).toHaveBeenCalledWith("/test");
});
