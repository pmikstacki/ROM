import { test, expect, type Page } from "@playwright/test";

async function settingsSession(page: Page, enabled = true) {
  const requests: string[] = [];
  let conflict = false;
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({
      json: {
        authenticated: true,
        generation: "settings-session",
        csrf_token: "csrf",
        user_id: "admin",
        expires_at: Math.floor(Date.now() / 1000) + 60,
      },
    }),
  );
  await page.route("**/rom-studio/api/**", (route) => {
    const endpoint = route.request().url().split("/").at(-1);
    const body = route.request().postDataJSON();
    requests.push(route.request().postData()!);
    const ordinary = {
      kind: "Task",
      version: 1,
      fields: [{ name: "name", shape: { type: "string" } }],
      actions: [],
      action_inputs: [],
    };
    const settings = {
      ...ordinary,
      kind: "PluginPreferences",
      presentation: {
        label: "Mail preferences",
        title_field: "name",
        settings: { group: "mail-plugin", label: "Mail plugin" },
      },
    };
    if (endpoint === "discover")
      return route.fulfill({
        json: {
          version: 1,
          resources: [
            ordinary,
            { ...ordinary, kind: "LooksLikeSettings" },
            ...(enabled ? [settings] : []),
          ],
        },
      });
    if (endpoint === "invoke" && conflict)
      return route.fulfill({
        status: 409,
        json: { error: "revision conflict" },
      });
    const row = {
      key: { kind: body.kind, id: "one" },
      revision: 7,
      value: { name: "Delivery defaults" },
    };
    return route.fulfill({ json: endpoint === "query" ? [row] : row });
  });
  await page.goto("./");
  await expect(
    page.getByRole("heading", { name: "Task", exact: true }),
  ).toBeVisible();
  return {
    requests,
    setConflict: () => {
      conflict = true;
    },
  };
}

test("Settings groups disclosed plugin Resources and uses the authorized patch path", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  const session = await settingsSession(page);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  const groups = page.getByRole("navigation", {
    name: "Settings groups",
    exact: true,
  });
  await expect(groups.getByText("Mail plugin", { exact: true })).toBeVisible();
  await expect(groups).not.toContainText("LooksLikeSettings");
  await expect(groups).not.toContainText("Hidden plugin");
  await expect(
    page.getByRole("cell", { name: "Delivery defaults", exact: true }).first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "Open one", exact: true }).click();
  const input = page.getByRole("textbox", { name: "name value", exact: true });
  await input.fill("Edited delivery defaults");
  await page.getByRole("tab", { name: "Filters", exact: true }).click();
  await page.getByRole("tab", { name: "Details", exact: true }).click();
  await expect(input).toHaveValue("Edited delivery defaults");
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(input).toHaveValue("Edited delivery defaults");
  await page.setViewportSize({ width: 1440, height: 1000 });
  session.setConflict();
  await page.getByRole("button", { name: "Save 1 change", exact: true }).click();
  await expect(
    page.getByRole("alert").filter({ hasText: "revision conflict" }).first(),
  ).toBeVisible();
  const invocation = session.requests
    .map((text) => JSON.parse(text))
    .find((body) => body.operation);
  expect(invocation).toMatchObject({
    kind: "PluginPreferences",
    id: "one",
    expected: 7,
    operation: {
      type: "patch",
      input: { name: { op: "set", value: "Edited delivery defaults" } },
    },
  });
  await expect(input).toHaveValue("Edited delivery defaults");
});

test("Settings exposes no group when discovery discloses no settings metadata", async ({
  page,
}) => {
  await settingsSession(page, false);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(
    page.getByText("No Settings available", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("navigation", { name: "Settings groups", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "LooksLikeSettings", exact: true }),
  ).toHaveCount(0);
});
