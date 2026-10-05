import { test, expect } from "@playwright/test";
// This trusted verifier fixture is copied separately from the public consumer source.
import { startHost } from "../../studio/tests/runtime/host-fixture.mjs";
for (const backend of ["sqlite", "redb"])
  test(`external author renderer and generic ${backend} flow`, async ({ page }) => {
    const host = await startHost(backend);
    console.log(JSON.stringify({ backend, fixture_directory: host.directory }));
    test.info().annotations.push({ type: "retained-fixture", description: host.directory });
    try {
      await page.goto(host.url);
      await page.getByRole("link", { name: "Sign in with Local fixture" }).click();
      await page.getByLabel("Fixture account").selectOption("alice");
      await page.getByRole("button", { name: "Sign in", exact: true }).click();
      await page.getByRole("button", { name: "Allow", exact: true }).click();
      await expect(page.getByRole("button", { name: "Sign out", exact: true })).toBeVisible();
      await page.getByRole("button", { name: "maintenance-tickets", exact: true }).click();
      await expect(page.getByText("Author code: TICKET-A1", { exact: true }).first()).toBeVisible();
      await page.getByRole("button", { name: "Open ticket-a", exact: true }).click();
      await page.getByRole("button", { name: "Edit code", exact: true }).click();
      await page.getByLabel("Author code editor", { exact: true }).fill("ticket-author2");
      await page.getByLabel("Author code editor", { exact: true }).press("Escape");
      await page.getByRole("button", { name: "Save 1 change", exact: true }).click();
      await expect(page.getByText("Author code: TICKET-AUTHOR2", { exact: true }).first()).toBeVisible();
      await expect(page.getByText("Revision 2", { exact: true })).toBeVisible();
      await page.getByRole("button", { name: "tasks", exact: true }).click();
      await page.getByRole("button", { name: "Open task-a", exact: true }).click();
      await page.getByLabel("title value", { exact: true }).fill("External author ordinary resource");
      await page.getByRole("button", { name: "Save 1 change", exact: true }).click();
      await expect(page.getByText("External author ordinary resource", { exact: true }).first()).toBeVisible();
    } finally { await host.close(); }
  });
