import { vi, it, expect } from "vitest";

const mocks = { myFunc: vi.fn() };
vi.mock("fs", () => ({ readFileSync: mocks.myFunc }));

it("works", () => {
  expect(mocks.myFunc).toBeDefined();
});
