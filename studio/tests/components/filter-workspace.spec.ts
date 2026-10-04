import { test, expect, type Page } from "@playwright/test";
import { selectValue } from "./select-value.ts";

async function connect(page: Page) {
  const queries: unknown[] = [];
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({
      json: {
        authenticated: true,
        generation: "filter-session",
        csrf_token: "csrf",
        user_id: "human",
        expires_at: Math.floor(Date.now() / 1000) + 60,
      },
    }),
  );
  await page.route("**/rom-studio/api/**", (route) => {
    const body = route.request().postDataJSON();
    const endpoint = route.request().url().split("/").at(-1);
    if (endpoint === "discover")
      return route.fulfill({
        json: {
          version: 1,
          resources: [
            {
              kind: "Task",
              version: 1,
              fields: [
                { name: "title", shape: { type: "string" } },
                { name: "done", shape: { type: "bool" } },
                { name: "count", shape: { type: "u64" } },
              ],
              actions: [],
              action_inputs: [],
            },
          ],
        },
      });
    if (endpoint === "query") queries.push(body.query);
    const row = {
      key: { kind: "Task", id: "one" },
      revision: 1,
      value: { title: "original", done: false },
    };
    return route.fulfill({ json: endpoint === "query" ? [row] : row });
  });
  await page.goto("./");
  await expect(
    page.getByRole("cell", { name: "original", exact: true }),
  ).toBeVisible();
  return queries;
}

test("compact actions keep their names and details stays beside quick filters", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await connect(page);
  const filters = page.getByRole("button", {
    name: "Quick filters",
    exact: true,
  });
  const details = page.getByRole("button", {
    name: "Open details panel",
    exact: true,
  });
  const actions = [
    "Create Resource",
    "Refresh",
    "Observe live query",
    "Open one",
    "First page",
    "Previous page",
    "Next page",
  ];
  for (const name of actions) {
    const button = page.getByRole("button", { name, exact: true });
    await expect(button).toBeVisible();
    await expect(button.locator("svg")).toHaveCount(1);
    await expect(button.locator("span")).toBeHidden();
  }
  const filterBox = await filters.boundingBox();
  const detailBox = await details.boundingBox();
  expect(filterBox).not.toBeNull();
  expect(detailBox).not.toBeNull();
  expect(Math.abs(filterBox!.y - detailBox!.y)).toBeLessThan(2);
  expect(detailBox!.x).toBeGreaterThan(filterBox!.x);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);

  await page.setViewportSize({ width: 1440, height: 900 });
  for (const name of actions) {
    await expect(
      page.getByRole("button", { name, exact: true }).locator("span"),
    ).toBeVisible();
  }
  await expect(details.locator("span")).toBeVisible();
});

test("quick and full filters share draft without querying until Apply", async ({
  page,
}) => {
  const queries = await connect(page);
  await page
    .getByRole("button", { name: "Quick filters", exact: true })
    .click();
  const quick = page.getByRole("dialog", {
    name: "Quick filters",
    exact: true,
  });
  await expect(quick).toBeVisible();
  await quick
    .getByRole("button", { name: "Add condition", exact: true })
    .click();
  await selectValue(
    quick.getByRole("button", { name: "Filter 1 field", exact: true }),
    "title",
  );
  await quick
    .getByRole("textbox", { name: "Filter 1 value", exact: true })
    .fill("needle");
  expect(queries).toHaveLength(1);
  await quick
    .getByRole("button", { name: "Open full filters", exact: true })
    .click();
  await expect(quick).not.toBeVisible();
  const full = page.getByRole("region", { name: "Filter editor", exact: true });
  await expect(
    full.getByRole("textbox", { name: "Filter 1 value", exact: true }),
  ).toHaveValue("needle");
  expect(queries).toHaveLength(1);
  await full
    .getByRole("button", { name: "Apply filters", exact: true })
    .click();
  await expect.poll(() => queries.length).toBe(2);
  expect(queries[1]).toMatchObject({
    filters: [{ field: "title", value: "needle" }],
    limit: 50,
  });
  await expect(
    page.getByRole("group", { name: "Applied query", exact: true }),
  ).toContainText("needle");
  await full
    .getByRole("textbox", { name: "Filter 1 value", exact: true })
    .fill("draft only");
  await full
    .getByRole("button", { name: "Discard edits", exact: true })
    .click();
  await expect(
    full.getByRole("textbox", { name: "Filter 1 value", exact: true }),
  ).toHaveValue("needle");
  expect(queries).toHaveLength(2);
});

test("Resource draft survives filter tab, Apply and mobile inspector lifecycle", async ({
  page,
}) => {
  await connect(page);
  await page.getByRole("button", { name: "Open one", exact: true }).click();
  const details = page.getByRole("region", {
    name: "Resource details",
    exact: true,
  });
  await expect(details).toBeVisible();
  await expect(
    details.getByRole("textbox", { name: "title value", exact: true }),
  ).toHaveValue("original");
  await details
    .getByRole("textbox", { name: "title value", exact: true })
    .fill("keep my draft");
  await page.getByRole("tab", { name: "Filters", exact: true }).click();
  const full = page.getByRole("region", { name: "Filter editor", exact: true });
  await full
    .getByRole("button", { name: "Apply filters", exact: true })
    .click();
  await page.getByRole("tab", { name: "Details", exact: true }).click();
  await expect(
    details.getByRole("textbox", { name: "title value", exact: true }),
  ).toHaveValue("keep my draft");
  await page.setViewportSize({ width: 390, height: 844 });
  const sheet = page.getByRole("dialog", {
    name: "Edit Resource",
    exact: true,
  });
  await expect(sheet).toBeVisible();
  await expect(
    details.getByRole("textbox", { name: "title value", exact: true }),
  ).toHaveValue("keep my draft");
  await page.keyboard.press("Tab");
  expect(
    await page
      .locator(":focus")
      .evaluate((node) => !!node.closest('[data-slot="sheet-content"]')),
  ).toBe(true);
  await page.keyboard.press("Escape");
  await expect(sheet).not.toBeVisible();
  await expect(
    page.getByRole("button", { name: "Open details panel", exact: true }),
  ).toBeFocused();
  await page
    .getByRole("button", { name: "Open details panel", exact: true })
    .click();
  await page.getByRole("tab", { name: "Details", exact: true }).click();
  await expect(
    details.getByRole("textbox", { name: "title value", exact: true }),
  ).toHaveValue("keep my draft");
  await page.setViewportSize({ width: 1440, height: 1000 });
  await expect(
    details.getByRole("textbox", { name: "title value", exact: true }),
  ).toHaveValue("keep my draft");
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("invalid filter input survives popover close and switching editors", async ({
  page,
}) => {
  const queries = await connect(page);
  await page
    .getByRole("button", { name: "Quick filters", exact: true })
    .click();
  const quick = page.getByRole("dialog", {
    name: "Quick filters",
    exact: true,
  });
  await quick
    .getByRole("button", { name: "Add condition", exact: true })
    .click();
  await selectValue(
    quick.getByRole("button", { name: "Filter 1 field", exact: true }),
    "count",
  );
  await quick
    .getByRole("textbox", { name: "Filter 1 value", exact: true })
    .fill("not an integer");
  await expect(
    quick.getByRole("button", { name: "Apply filters", exact: true }),
  ).toBeDisabled();
  await quick
    .getByRole("button", { name: "Open full filters", exact: true })
    .click();
  const full = page.getByRole("region", { name: "Filter editor", exact: true });
  await expect(
    full.getByRole("textbox", { name: "Filter 1 value", exact: true }),
  ).toHaveValue("not an integer");
  await expect(
    full.getByRole("button", { name: "Apply filters", exact: true }),
  ).toBeDisabled();
  expect(queries).toHaveLength(1);
  await full
    .getByRole("textbox", { name: "Filter 1 value", exact: true })
    .fill("123");
  await expect(
    full.getByRole("button", { name: "Apply filters", exact: true }),
  ).toBeEnabled();
  await page
    .getByRole("button", { name: "Quick filters", exact: true })
    .click();
  await expect(
    quick.getByRole("textbox", { name: "Filter 1 value", exact: true }),
  ).toHaveValue("123");
  await quick
    .getByRole("textbox", { name: "Filter 1 value", exact: true })
    .fill("invalid again");
  await page.keyboard.press("Escape");
  await expect(
    full.getByRole("textbox", { name: "Filter 1 value", exact: true }),
  ).toHaveValue("invalid again");
  await full
    .getByRole("button", { name: "Clear filters", exact: true })
    .click();
  await expect(
    full.getByRole("button", { name: "Apply filters", exact: true }),
  ).toBeEnabled();
  expect(queries).toHaveLength(1);
});

test("removing an earlier rule preserves the later rule's invalid draft", async ({
  page,
}) => {
  const queries = await connect(page);
  const full = page.getByRole("region", { name: "Filter editor", exact: true });
  await full
    .getByRole("button", { name: "Add condition", exact: true })
    .click();
  await full
    .getByRole("button", { name: "Add condition", exact: true })
    .click();
  await selectValue(
    full.getByRole("button", { name: "Filter 2 field", exact: true }),
    "count",
  );
  await full
    .getByRole("textbox", { name: "Filter 2 value", exact: true })
    .fill("invalid later rule");
  await full
    .getByRole("button", { name: "Remove filter 1", exact: true })
    .click();
  await expect(
    full.getByRole("textbox", { name: "Filter 1 value", exact: true }),
  ).toHaveValue("invalid later rule");
  await expect(
    full.getByRole("button", { name: "Apply filters", exact: true }),
  ).toBeDisabled();
  await full
    .getByRole("textbox", { name: "Filter 1 value", exact: true })
    .fill("18446744073709551615");
  await expect(
    full.getByRole("button", { name: "Apply filters", exact: true }),
  ).toBeEnabled();
  expect(queries).toHaveLength(1);
});

test("descriptor change permits explicit discard of an obsolete filter", async ({
  page,
}) => {
  await page.goto("tests/components/details.html");
  await page.getByRole("tab", { name: "Filters", exact: true }).click();
  const full = page.getByRole("region", { name: "Filter editor", exact: true });
  await expect(
    full.getByRole("textbox", { name: "Filter 1 value", exact: true }),
  ).toHaveValue("original");
  await page
    .getByRole("button", { name: "Remove field definition", exact: true })
    .click();
  await expect(
    full.getByRole("button", { name: "Apply filters", exact: true }),
  ).toBeDisabled();
  await expect(
    full.getByRole("button", { name: "Discard edits", exact: true }),
  ).toBeEnabled();
  await full
    .getByRole("button", { name: "Discard edits", exact: true })
    .click();
  await expect(
    full.getByRole("textbox", { name: "Filter 1 value", exact: true }),
  ).toHaveCount(0);
  await expect(
    full.getByRole("button", { name: "Apply filters", exact: true }),
  ).toBeEnabled();
  await expect(page.getByRole("alert")).toContainText(
    "current query uses unavailable fields",
  );
});

test("invalid input after an applied numeric filter is marked pending", async ({
  page,
}) => {
  const queries = await connect(page);
  const full = page.getByRole("region", { name: "Filter editor", exact: true });
  await full
    .getByRole("button", { name: "Add condition", exact: true })
    .click();
  await selectValue(
    full.getByRole("button", { name: "Filter 1 field", exact: true }),
    "count",
  );
  await full
    .getByRole("textbox", { name: "Filter 1 value", exact: true })
    .fill("123");
  await full
    .getByRole("button", { name: "Apply filters", exact: true })
    .click();
  await expect.poll(() => queries.length).toBe(2);
  await expect(full.getByText("Applied", { exact: true })).toBeVisible();
  await full
    .getByRole("textbox", { name: "Filter 1 value", exact: true })
    .fill("invalid pending edit");
  await expect(full.getByText("Pending edits", { exact: true })).toBeVisible();
  await expect(
    full.getByRole("button", { name: "Apply filters", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("group", { name: "Applied query", exact: true }),
  ).toContainText("123");
  expect(queries).toHaveLength(2);
});
