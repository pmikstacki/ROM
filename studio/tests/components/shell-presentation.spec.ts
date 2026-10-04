import { test, expect } from "@playwright/test";

test("mobile sidebar opens, selects a descriptor and returns space to the page", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({
      json: {
        authenticated: true,
        generation: "shell-session",
        csrf_token: "csrf",
        user_id: "human",
        expires_at: Math.floor(Date.now() / 1000) + 60,
      },
    }),
  );
  await page.route("**/rom-studio/api/**", (route) => {
    const body = route.request().postDataJSON();
    return route.fulfill({
      json: route.request().url().endsWith("discover")
        ? {
            version: 1,
            resources: ["Task", "Note"].map((kind) => ({
              kind,
              version: 1,
              fields: [{ name: "title", shape: { type: "string" } }],
              actions: [],
              action_inputs: [],
            })),
          }
        : [
            {
              key: { kind: body.kind, id: "one" },
              revision: 1,
              value: { title: "Visible row" },
            },
          ],
    });
  });
  await page.goto("./");
  await expect(
    page.getByRole("heading", { name: "Task", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Note", exact: true }),
  ).not.toBeVisible();
  await page.getByRole("button", { name: "Toggle Sidebar" }).click();
  await expect(
    page.getByRole("dialog", { name: "Sidebar", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Tab");
  expect(
    await page
      .getByRole("dialog", { name: "Sidebar", exact: true })
      .evaluate((element) => element.contains(document.activeElement)),
  ).toBe(true);
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("dialog", { name: "Sidebar", exact: true }),
  ).not.toBeVisible();
  await expect(
    page.getByRole("button", { name: "Toggle Sidebar" }),
  ).toBeFocused();
  await page.getByRole("button", { name: "Toggle Sidebar" }).click();
  await page.getByRole("button", { name: "Note", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Note", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Sign out" }),
  ).not.toBeVisible();
  await expect(page.getByRole("cell", { name: "Visible row" })).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});
