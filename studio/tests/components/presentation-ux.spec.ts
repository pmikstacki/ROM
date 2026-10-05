import { test, expect } from "@playwright/test";

const identity = '["local","human","alice"]';

test.beforeEach(async ({ page }) => {
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({
      json: {
        authenticated: true,
        generation: "presentation",
        csrf_token: "csrf",
        user_id: "alice",
        expires_at: Math.floor(Date.now() / 1000) + 60,
      },
    }),
  );
  await page.route("**/rom-studio/api/**", (route) => {
    const endpoint = route.request().url().split("/").at(-1);
    if (endpoint === "discover")
      return route.fulfill({
        json: {
          version: 1,
          resources: [
            {
              kind: "users",
              version: 1,
              fields: [{ name: "display_name", shape: { type: "string" } }],
              actions: [],
              action_inputs: [],
              presentation: {
                label: "User",
                title_field: "display_name",
                fields: {
                  display_name: {
                    label: "Display name",
                    help: "A name visible to other users.",
                  },
                },
              },
            },
          ],
        },
      });
    const row = {
      key: { kind: "users", id: identity },
      revision: 1,
      value: { display_name: "Alice" },
    };
    return route.fulfill({ json: endpoint === "query" ? [row] : row });
  });
});

test("generic Resource presentation separates human title, field label, and exact identity", async ({
  page,
}) => {
  await page.goto("./");
  await page
    .getByRole("button", { name: `Open ${identity}`, exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Alice", exact: true }),
  ).toBeVisible();
  await expect(page.getByLabel("Resource ID", { exact: true })).toHaveText(
    identity,
  );
  await expect(
    page.getByRole("columnheader", { name: "Display name", exact: true }),
  ).toBeVisible();
  await expect(
    page
      .getByRole("group", { name: "Display name", exact: true })
      .getByText("Display name", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("textbox", { name: "Display name value", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Copy Resource ID", exact: true }),
  ).toBeVisible();
});

test("mobile Resource details returns focus to the actual row opener", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("./");
  const opener = page.getByRole("button", {
    name: `Open ${identity}`,
    exact: true,
  });
  await opener.focus();
  await opener.press("Enter");
  await expect(
    page.getByRole("dialog", { name: "Edit Resource", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(opener).toBeFocused();
});
