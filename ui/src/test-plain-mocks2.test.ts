import { vi, it, expect } from "vitest";

const mocks = vi.hoisted(() => ({ myFunc: vi.fn() }));
vi.mock("path", () => ({ join: mocks.myFunc }));
import { join } from "path";

it("works", () => {
  expect(join).toBe(mocks.myFunc);
});
