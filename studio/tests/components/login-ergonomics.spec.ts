import { test, expect } from "@playwright/test";

test("sign-in providers stay readable on narrow screens and preserve exact login route", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({ json: { authenticated: false, generation: "signed-out" } }),
  );
  await page.route("**/rom-studio/auth/providers", (route) =>
    route.fulfill({
      json: {
        providers: [
          {
            id: "long-provider",
            label:
              "Example organization with a very long identity provider name",
          },
        ],
        primary: null,
      },
    }),
  );
  await page.goto("./");
  const provider = page.getByRole("link", {
    name: "Sign in with Example organization with a very long identity provider name",
  });
  await expect(provider).toBeVisible();
  await expect(provider).toHaveAttribute(
    "href",
    "/rom-studio/auth/login/long-provider",
  );
  await provider.focus();
  await expect(provider).toBeFocused();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await expect(
    page.getByRole("button", { name: "Check connection", exact: true }),
  ).toBeVisible();
  await page.evaluate(() => {
    document.body.style.zoom = "2";
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await expect(provider).toBeVisible();
});
test("unavailable session presents retry without exposing workspace values", async ({
  page,
}) => {
  let requests = 0;
  await page.route("**/rom-studio/auth/session", (route) => {
    requests++;
    return route.fulfill({ status: 503, json: { error: "down" } });
  });
  await page.goto("./");
  await expect(page.getByRole("alert")).toHaveText("Session unavailable.");
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: "No sign-in providers are available" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Check connection", exact: true })
    .click();
  await expect.poll(() => requests).toBe(2);
  await expect(page.getByRole("table")).not.toBeVisible();
});
test("primary provider auto-redirect retains explicit sign-in choice", async ({
  page,
}) => {
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({ json: { authenticated: false, generation: "signed-out" } }),
  );
  await page.route("**/rom-studio/auth/providers", (route) =>
    route.fulfill({
      json: {
        providers: [{ id: "preferred", label: "Organization" }],
        primary: "preferred",
      },
    }),
  );
  await page.route("**/rom-studio/auth/login/preferred", (route) =>
    route.fulfill({
      contentType: "text/html",
      body: "<main>Identity provider reached</main>",
    }),
  );
  await page.goto("./");
  await expect(page).toHaveURL(/\/auth\/login\/preferred$/);
  await expect(page.getByText("Identity provider reached")).toBeVisible();
  await page.goto("./");
  await expect(
    page.getByRole("link", { name: "Sign in with Organization" }),
  ).toBeVisible();
  await expect(
    page.getByText("Preferred sign-in provider", { exact: true }),
  ).toBeVisible();
});
test("expired session clears protected values and explains sign-in", async ({
  page,
}) => {
  let authenticated = true;
  await page.clock.install();
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({
      json: authenticated
        ? {
            authenticated: true,
            generation: "session",
            csrf_token: "csrf",
            user_id: "human",
            expires_at: Math.floor(Date.now() / 1000) + 3600,
          }
        : { authenticated: false, generation: "expired-session" },
    }),
  );
  await page.route("**/rom-studio/auth/providers", (route) =>
    route.fulfill({
      json: {
        providers: [{ id: "organization", label: "Organization" }],
        primary: null,
      },
    }),
  );
  await page.route("**/rom-studio/api/**", (route) =>
    route.fulfill({
      json: route.request().url().endsWith("discover")
        ? {
            version: 1,
            resources: [
              {
                kind: "reports",
                version: 1,
                fields: [{ name: "title", shape: { type: "string" } }],
                actions: [],
                action_inputs: [],
              },
            ],
          }
        : [
            {
              key: { kind: "reports", id: "private" },
              revision: 1,
              value: { title: "PROTECTED-REPORT" },
            },
          ],
    }),
  );
  await page.goto("./");
  await expect(
    page.getByRole("cell", { name: "PROTECTED-REPORT", exact: true }),
  ).toBeVisible();
  authenticated = false;
  await page.clock.fastForward(15001);
  await expect(
    page.getByText("Your session ended. Sign in again to continue."),
  ).toBeVisible();
  await expect(
    page.getByText("PROTECTED-REPORT", { exact: true }),
  ).not.toBeVisible();
  await expect(
    page.getByRole("link", { name: "Sign in with Organization" }),
  ).toBeVisible();
});

test("expired authenticated response retains available provider sign-in", async ({
  page,
}) => {
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({
      json: {
        authenticated: true,
        generation: "expired",
        csrf_token: "expired-csrf",
        user_id: "human",
        expires_at: Math.floor(Date.now() / 1000) - 1,
      },
    }),
  );
  await page.route("**/rom-studio/auth/providers", (route) =>
    route.fulfill({
      json: {
        providers: [{ id: "organization", label: "Organization" }],
        primary: null,
      },
    }),
  );
  await page.goto("./");
  await expect(
    page.getByText("Your session ended. Sign in again to continue."),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Sign in with Organization" }),
  ).toBeVisible();
  await expect(page.getByRole("table")).not.toBeVisible();
});
