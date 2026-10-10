import { test, expect } from "@playwright/test";

for (const width of [1440, 390]) {
  test(`installed root App and renderer compose resources settings and work at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 960 });
    await page.route("**/auth/session", route => route.fulfill({ json: {
      authenticated: true, generation: "installed-root-session", csrf_token: "fixture-csrf",
      user_id: "fixture-human", expires_at: Math.floor(Date.now() / 1000) + 120,
    } }));
    await page.route("**/api/**", route => {
      const url = route.request().url();
      const descriptor = { kind: "Notes", version: 1, fields: [
        { name: "title", shape: { type: "string" }, codec: { name: "installed-title", version: 1 } },
      ], actions: [], action_inputs: [] };
      if (url.endsWith("/discover")) return route.fulfill({ json: { version: 1, resources: [descriptor, {
        ...descriptor, kind: "PluginPreferences", presentation: { label: "Plugin preferences", settings: { group: "maintenance", label: "Maintenance" } },
      }] } });
      if (url.endsWith("/work/capabilities")) return route.fulfill({ json: { protocol_version: 1, inspect: true, retry: false, reconcile: false } });
      if (url.endsWith("/work/list")) return route.fulfill({ json: { protocol_version: 1, records: [], cursor: null } });
      const request = route.request().postDataJSON();
      if (url.endsWith("/read") || url.endsWith("/query")) {
        const row = { key: { kind: request.kind, id: "one" }, revision: 1, value: { title: `${request.kind} title` } };
        return route.fulfill({ json: url.endsWith("/query") ? [row] : row });
      }
      return route.fulfill({ status: 400, json: { category: "invalid", error: "Unexpected fixture operation." } });
    });
    await page.goto("/?studio");
    await expect(page.getByRole("heading", { name: "Notes", exact: true })).toBeVisible();
    await expect(page.getByTestId("installed-title")).toHaveText("Notes title");
    await page.getByRole("button", { name: "Open one", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Resource one", exact: true })).toBeVisible();
    if (width < 768) await page.getByRole("dialog").getByRole("button", { name: "Close", exact: true }).click();
    else await page.getByRole("button", { name: "Close details panel", exact: true }).click();
    async function navigate(name: string) {
      const button = page.getByRole("button", { name, exact: true });
      if (width < 768) {
        await expect(page.getByRole("dialog")).toHaveCount(0);
        await page.getByRole("button", { name: "Toggle Sidebar", exact: true }).click();
        await expect(page.getByRole("dialog")).toBeVisible();
      }
      await button.click();
      if (width < 768) await expect(page.getByRole("dialog")).toHaveCount(0);
    }
    await navigate("Settings");
    await expect(page.getByRole("heading", { name: "Settings", exact: true })).toBeVisible();
    await expect(page.getByRole("navigation", { name: "Settings groups", exact: true }).getByText("Maintenance", { exact: true })).toBeVisible();
    await navigate("Work");
    await expect(page.getByRole("heading", { name: "Work", exact: true })).toBeVisible();
    await expect(page.getByText("No work matches the applied filters.", { exact: true })).toBeVisible();
    await expect(page.getByRole("alert")).toHaveCount(0);
  });
}
