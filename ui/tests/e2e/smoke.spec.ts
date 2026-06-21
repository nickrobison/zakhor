import { test, expect } from "@playwright/test";

test("smoke", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: /zakhor web ui/i })).toBeVisible();
});
