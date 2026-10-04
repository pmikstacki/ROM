import { test, expect } from "@playwright/test";
const descriptor = (kind: string) => ({
  kind,
  version: 1,
  fields: [{ name: "title", shape: { type: "string" } }],
  actions: [],
  action_inputs: [],
});
test("real SDK composes two metadata-driven pages; logout clears details", async ({
  page,
}) => {
  let authenticated = true;
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({
      json: {
        authenticated,
        generation: "fixture-session",
        ...(authenticated
          ? {
              csrf_token: "fixture-csrf",
              user_id: "fixture-human",
              expires_at: Math.floor(Date.now() / 1000) + 60,
            }
          : {}),
      },
    }),
  );
  await page.route("**/rom-studio/auth/providers", (route) =>
    route.fulfill({
      json: {
        providers: [{ id: "fixture", label: "Test provider" }],
        primary: null,
      },
    }),
  );
  await page.route("**/rom-studio/auth/logout", (route) => {
    authenticated = false;
    return route.fulfill({ status: 204 });
  });
  await page.route("**/rom-studio/api/**", async (route) => {
    const request = route.request(),
      body = request.postDataJSON();
    const endpoint = request.url().split("/").at(-1);
    await route.fulfill({
      json:
        endpoint === "discover"
          ? { version: 1, resources: [descriptor("Task"), descriptor("Note")] }
          : endpoint === "query"
            ? [
                {
                  key: { kind: body.kind, id: "one" },
                  revision: 1,
                  value: { title: `${body.kind} value` },
                },
              ]
            : {
                key: { kind: body.kind, id: body.id },
                revision: 1,
                value: { title: `${body.kind} value` },
              },
    });
  });
  await page.goto("./");
  await expect(
    page.getByRole("heading", { name: "Task", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Task", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await page.getByRole("button", { name: "Open one" }).click();
  await expect(
    page.getByRole("heading", { name: "Resource one" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Note", exact: true }).click();
  await expect(
    page.getByRole("cell", { name: "Note value", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Note", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await expect(
    page.getByRole("button", { name: "Task", exact: true }),
  ).not.toHaveAttribute("aria-current", "page");
  await expect(
    page.getByRole("heading", { name: "Resource one" }),
  ).not.toBeVisible();
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(
    page.getByRole("link", { name: "Sign in with Test provider" }),
  ).toBeVisible();
  await expect(page.getByText("Note value", { exact: true })).not.toBeVisible();
});
test("real app reports unavailable session without simulated data", async ({
  page,
}) => {
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({ status: 503, json: { error: "down" } }),
  );
  await page.goto("./");
  await expect(page.getByRole("alert")).toHaveText("Session unavailable.");
  await expect(
    page.getByRole("heading", { name: "Connect to ROM" }),
  ).toBeVisible();
});

test("moving pages use native query anchors and previous refetches", async ({
  page,
}) => {
  let queryCalls = 0,
    anchorCalls = 0;
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({
      json: {
        authenticated: true,
        generation: "fixture-pagination",
        csrf_token: "csrf",
        user_id: "human",
        expires_at: Math.floor(Date.now() / 1000) + 60,
      },
    }),
  );
  await page.route("**/rom-studio/api/**", async (route) => {
    const body = route.request().postDataJSON(),
      endpoint = route.request().url().split("/").at(-1);
    if (endpoint === "discover")
      return route.fulfill({
        json: { version: 1, resources: [descriptor("Task")] },
      });
    if (endpoint === "anchor") {
      anchorCalls++;
      return route.fulfill({
        json: {
          version: 1,
          kind: "Task",
          schema_version: 1,
          filters: [],
          comparisons: [],
          order: [],
          id: "one",
          values: [],
        },
      });
    }
    queryCalls++;
    return route.fulfill({
      json: [
        {
          key: { kind: "Task", id: body.query.after ? "two" : "one" },
          revision: 1,
          value: { title: body.query.after ? "second page" : "first page" },
        },
      ],
    });
  });
  await page.goto("./");
  await expect(
    page.getByRole("heading", { name: "Task", exact: true }),
  ).toBeVisible();
  await page.getByLabel("Query limit").fill("1");
  await page.getByRole("button", { name: "Apply filters" }).click();
  await page.getByRole("button", { name: "Next page", exact: true }).click();
  await expect(page.getByText("Moving page 2")).toBeVisible();
  await expect(page.getByRole("cell", { name: "second page" })).toBeVisible();
  await page.getByRole("button", { name: "Previous page" }).click();
  await expect(page.getByRole("cell", { name: "first page" })).toBeVisible();
  await expect(page.getByText("Moving page 1")).toBeVisible();
  expect(anchorCalls).toBe(1);
  expect(queryCalls).toBe(4);
});
