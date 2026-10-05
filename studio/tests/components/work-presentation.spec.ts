import { test, expect, type Page } from "@playwright/test";
const handle = "a".repeat(64);
const work = {
  protocol_version: 1,
  handle,
  version: { generation: "history", revision: 2 },
  category: "Reaction",
  definition: { name: "Notify assignee", version: 1 },
  state: "AwaitingReconciliation",
  attempts: 3,
  due: 9007199254740993n.toString(),
  delivery: "Unknown",
  source: { kind: "Task", id: "task-one" },
  target: null,
};
async function fixture(page: Page) {
  const queries: Record<string, unknown>[] = [],
    controls: Record<string, unknown>[] = [];
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({
      json: {
        authenticated: true,
        generation: "work-session",
        csrf_token: "csrf",
        user_id: "operator",
        expires_at: Math.floor(Date.now() / 1000) + 60,
      },
    }),
  );
  await page.route("**/rom-studio/api/**", async (route) => {
    const url = route.request().url(),
      body = route.request().postDataJSON();
    if (url.endsWith("/discover"))
      return route.fulfill({ json: { version: 1, resources: [] } });
    if (url.endsWith("/work/capabilities"))
      return route.fulfill({
        json: {
          protocol_version: 1,
          inspect: true,
          retry: true,
          reconcile: true,
        },
      });
    // Deliver an exact integer token, rather than a lossy JS numeric fixture.
    const view = JSON.stringify(work).replace(
      '"due":"9007199254740993"',
      '"due":9007199254740993',
    );
    if (url.endsWith("/work/list")) {
      queries.push(body);
      return route.fulfill({
        contentType: "application/json",
        body: `{"protocol_version":1,"records":[${view}],"cursor":null}`,
      });
    }
    if (url.endsWith("/work/read"))
      return route.fulfill({ contentType: "application/json", body: view });
    if (url.endsWith("/work/control")) {
      controls.push(body);
      if (controls.length === 1) return route.abort("failed");
      return route.fulfill({
        json: {
          protocol_version: 1,
          handle,
          version: { generation: "history", revision: 3 },
          key: body.key,
          operation: body.operation,
          outcome: "Scheduled",
          replayed: true,
        },
      });
    }
    throw Error(`Unexpected request ${url}`);
  });
  await page.goto("./");
  await page.getByRole("button", { name: "Work", exact: true }).click();
  return { queries, controls };
}
test("Work loads readable snapshots automatically and applies operator filters", async ({
  page,
}) => {
  const { queries } = await fixture(page);
  await expect(
    page.getByRole("button", { name: "Inspect Notify assignee" }),
  ).toBeVisible();
  expect(queries).toEqual([{ limit: 50 }]);
  await page.getByRole("button", { name: "Open details panel" }).click();
  await page.getByLabel("Work definition").fill("Notify assignee");
  await page.getByRole("button", { name: "Apply work filters" }).click();
  expect(queries).toEqual([
    { limit: 50 },
    { limit: 50, definition: "Notify assignee" },
  ]);
  await page.getByRole("button", { name: "Inspect Notify assignee" }).click();
  await expect(
    page.getByRole("heading", { name: "Notify assignee", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("9007199254740993", { exact: true }).last(),
  ).toBeVisible();
  await expect(
    page.getByText("Awaiting reconciliation", { exact: true }).last(),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Copy work handle" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Retry work", exact: true }),
  ).toBeDisabled();
});
test("Work inspector close preserves the exact recovery retry request", async ({
  page,
}) => {
  const { controls } = await fixture(page);
  await page.getByRole("button", { name: "Inspect Notify assignee" }).click();
  await page
    .getByRole("checkbox", { name: "I confirm this recovery action" })
    .check();
  await page.getByRole("button", { name: "Retry work", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Retry same recovery request" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Resources", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Close details panel" }).click();
  await page.getByRole("button", { name: "Open details panel" }).click();
  await page
    .getByRole("button", { name: "Retry same recovery request" })
    .click();
  await expect(
    page.getByRole("status").filter({ hasText: "Scheduled" }),
  ).toBeVisible();
  expect(controls).toHaveLength(2);
  expect(controls[1]).toEqual(controls[0]);
  await expect(
    page.getByRole("button", { name: "Resources", exact: true }),
  ).toBeEnabled();
  await expect(
    page.getByRole("button", { name: "Retry work", exact: true }),
  ).toBeDisabled();
});

test("mobile Work details returns focus to the actual row opener", async ({
  page,
}) => {
  await fixture(page);
  await page.setViewportSize({ width: 390, height: 844 });
  const inspect = page.getByRole("button", { name: "Inspect Notify assignee" });
  await inspect.focus();
  await inspect.press("Enter");
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Notify assignee", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await expect(inspect).toBeFocused();
  const toggle = page.getByRole("button", { name: "Open details panel" });
  await toggle.press("Enter");
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect
    .poll(() =>
      page
        .getByRole("dialog")
        .evaluate((dialog) => dialog.contains(document.activeElement)),
    )
    .toBe(true);
  await page.keyboard.press("Escape");
  await expect(toggle).toBeFocused();
});
