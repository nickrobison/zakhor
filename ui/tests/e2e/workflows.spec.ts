import { expect, test } from "@playwright/test";
import { mockApi } from "./api-mocks";

test.describe("Zakhor web UI workflows", () => {
  test.beforeEach(async ({ page }) => {
    await mockApi(page);
  });

  test("home dashboard loads status, stats, recent activity, and quick search", async ({ page }) => {
    await page.goto("/");

    await expect(page.getByRole("heading", { name: /zakhor web ui/i })).toBeVisible();
    await expect(page.getByText("Ready")).toBeVisible();
    await expect(page.getByRole("link", { name: "Decisions", exact: true })).toBeVisible();
    await expect(page.getByText("3")).toBeVisible();
    await expect(page.getByText("Use React Flow for graph visualization")).toBeVisible();

    await page.getByPlaceholder("Search decisions, entities, and observations").fill("react flow");
    await page.getByRole("button", { name: "Search" }).click();

    await expect(page).toHaveURL(/\/search\?q=react(\+|%20)flow&mode=hybrid&type=all/);
    await expect(page.getByRole("heading", { name: /search/i })).toBeVisible();
  });

  test("search page filters results by type and changes ranking mode", async ({ page }) => {
    await page.goto("/search?q=react&mode=hybrid&type=all&limit=5");

    await expect(page.getByRole("heading", { name: /search/i })).toBeVisible();
    await expect(page.getByText("decision:react-flow")).toBeVisible();
    await expect(page.getByText("entity:graph-viz")).toBeVisible();
    await expect(page.getByText("observation:tracker-memory")).toBeVisible();

    await page.getByRole("tab", { name: "Decisions" }).click();
    await expect(page.getByText("decision:react-flow")).toBeVisible();
    await expect(page.getByText("entity:graph-viz")).not.toBeVisible();

    await page.getByRole("button", { name: "Semantic" }).click();
    await expect(page.getByRole("button", { name: "Semantic" })).toHaveAttribute("aria-pressed", "true");
  });

  test("decision explorer filters, opens detail, expands provenance, and renders graph", async ({ page }) => {
    await page.goto("/decisions");

    await expect(page.getByRole("heading", { name: /decision explorer/i })).toBeVisible();
    await expect(page.getByText("Use React Flow for graph visualization")).toBeVisible();

    await page.getByRole("button", { name: "Archived" }).click();
    await expect(page.getByText("Archive old design")).toBeVisible();

    await page.getByRole("button", { name: "Active" }).click();
    await page.getByRole("link", { name: "Use React Flow for graph visualization" }).click();

    await expect(page.getByRole("heading", { name: /decision detail/i })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Rationale" })).toBeVisible();
    await expect(page.getByText("React Flow provides the best balance of features and performance", { exact: true })).toBeVisible();
    await expect(page.getByText("Initial research on graph libraries")).toBeVisible();

    await page.getByRole("button", { name: /provenance step: observation-1/i }).click();
    await expect(page.getByText(/^Source:/)).toBeVisible();

    await page.getByRole("tab", { name: "Graph" }).click();
    await expect(page.getByRole("tab", { name: "Graph" })).toHaveAttribute("data-state", "active");
    await expect(page.getByRole("link", { name: "React Flow attribution" })).toBeVisible();
  });

  test("entity explorer opens detail and shows relationships, decisions, and observations", async ({ page }) => {
    await page.goto("/entities");

    await expect(page.getByRole("heading", { name: /entity explorer/i })).toBeVisible();
    await expect(page.getByRole("link", { name: "React Flow" })).toBeVisible();
    await expect(page.getByText("Technology")).toBeVisible();

    await page.getByRole("link", { name: "React Flow" }).click();

    await expect(page.getByRole("heading", { name: /entity detail/i })).toBeVisible();
    await expect(page.getByText("Entity ID: entity:react-flow", { exact: true })).toBeVisible();
    await expect(page.getByText("used by", { exact: true })).toBeVisible();

    await page.getByRole("tab", { name: "Decisions" }).click();
    await expect(page.getByText("Use React Flow for graph visualization")).toBeVisible();

    await page.getByRole("tab", { name: "Observations" }).click();
    await expect(page.getByText("React Flow was selected for the graph UI")).toBeVisible();
  });

  test("observation timeline filters, sorts, and paginates", async ({ page }) => {
    await page.goto("/observations?limit=1");

    await expect(page.getByRole("heading", { name: /observation timeline/i })).toBeVisible();
    await expect(page.getByText("Graph Explorer should use React Flow")).toBeVisible();
    await expect(page.getByText("Total: 2")).toBeVisible();

    await page.getByPlaceholder("Filter by entity ID").fill("entity:react-flow");
    await expect(page).toHaveURL(/entity_id=entity%3Areact-flow/);

    await page.getByRole("button", { name: "Oldest" }).click();
    await expect(page).toHaveURL(/sort=oldest/);

    await page.getByRole("button", { name: "Next" }).click();
    await expect(page.getByText("Page 2 of 2")).toBeVisible();
  });

  test("graph explorer traverses mocked graph and shows depth warning", async ({ page }) => {
    await page.goto("/graph");

    await expect(page.getByRole("heading", { name: /graph explorer/i })).toBeVisible();

    await page.getByLabel("Start node URI").fill("entity:react-flow");
    await page.getByRole("button", { name: "Traverse" }).click();

    await expect(page.getByRole("heading", { name: "entity:react-flow" })).toBeVisible();

    await page.getByLabel("Depth").selectOption("3");
    await page.getByRole("button", { name: "Traverse" }).click();

    await expect(page.getByText("Large graph — consider narrowing search.")).toBeVisible();
  });

  test("code integration searches and switches repository, file, and symbol tabs", async ({ page }) => {
    await page.goto("/code");

    await expect(page.getByRole("heading", { name: /code integration/i })).toBeVisible();
    await expect(page.getByText("No code references found for an entity ID or search query")).toBeVisible();

    await page.getByLabel("Entity ID or search query").fill("entity:react-flow");
    await page.getByRole("button", { name: "Search" }).click();

    await expect(page.getByRole("tab", { name: "Repositories" })).toBeVisible();
    await expect(page.getByText("zakhor/zakhor")).toBeVisible();

    await page.getByRole("tab", { name: "Files" }).click();
    await expect(page.getByText("src/components/graph/GraphView.tsx")).toBeVisible();

    await page.getByRole("tab", { name: "Symbols" }).click();
    await expect(page.getByText("GraphView", { exact: true })).toBeVisible();
  });

  test("administration console shows status, confirms rebuild, and updates success state", async ({ page }) => {
    await page.goto("/admin");

    await expect(page.getByRole("heading", { name: /administration console/i })).toBeVisible();
    await expect(page.getByText("ok", { exact: true })).toBeVisible();
    await expect(page.getByText("Available", { exact: true })).toBeVisible();
    await expect(page.getByText("1,280", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("960", { exact: true }).first()).toBeVisible();

    page.once("dialog", async (dialog) => {
      expect(dialog.message()).toContain("Rebuild lexical and semantic indexes");
      await dialog.accept();
    });

    await page.getByRole("button", { name: "Rebuild indexes" }).click();

    await expect(page.getByText("Rebuild accepted. Status will refresh automatically.")).toBeVisible();
  });

  test("empty API states render without uncaught failures", async ({ page }) => {
    await page.unroute("**/api/v1/**");
    await page.route("**/api/v1/search*", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ results: [], count: 0, warning: "No results" }),
      });
    });

    await page.goto("/search?q=empty-result");

    await expect(page.getByText("No results found for this query and filter.")).toBeVisible();
    await expect(page.getByText("No results", { exact: true })).toBeVisible();
  });
});
