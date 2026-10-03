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
  await page.getByRole("button", { name: "Open one" }).click();
  await expect(
    page.getByRole("heading", { name: "Resource one" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Note", exact: true }).click();
  await expect(
    page.getByRole("cell", { name: "Note value", exact: true }),
  ).toBeVisible();
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
