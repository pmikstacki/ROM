import { test, expect } from "@playwright/test";

test("installed details retain a draft across responsive transitions and return focus", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/?ui");
  await page.getByRole("button", { name: "Inspect equipment", exact: true }).click();
  await expect(page.getByRole("region", { name: "Equipment inspection", exact: true })).toBeVisible();
  await page.getByLabel("Inspection notes").fill("preserved raw draft");
  await page.setViewportSize({ width: 390, height: 844 });
  const dialog = page.getByRole("dialog", { name: "Equipment inspection", exact: true });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByLabel("Inspection notes")).toHaveValue("preserved raw draft");
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(page.getByRole("button", { name: "Inspect equipment", exact: true })).toBeFocused();
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.getByRole("button", { name: "Inspect equipment", exact: true }).click();
  await expect(page.getByLabel("Inspection notes")).toHaveValue("preserved raw draft");
});

test("installed headless requests fence late previews and retain exact multi-selection", async ({ page }) => {
  await page.goto("/?ui");
  await page.getByRole("button", { name: "Select pump", exact: true }).click();
  await page.getByRole("button", { name: "Select valve", exact: true }).click();
  await expect(page.getByLabel("Selected equipment")).toHaveText("pump-1,valve-1");
  await page.getByRole("button", { name: "Start slow preview", exact: true }).click();
  await expect(page.getByLabel("Preview", { exact: true })).toHaveText("pending");
  await page.getByRole("button", { name: "Load current preview", exact: true }).click();
  await expect(page.getByLabel("Preview", { exact: true })).toHaveText("current preview");
  await page.getByRole("button", { name: "Release old preview", exact: true }).click();
  await expect(page.getByLabel("Preview", { exact: true })).toHaveText("current preview");
});

test("installed observation removes prior private rows when authority generation changes", async ({ page }) => {
  await page.goto("/?ui");
  await expect(page.getByLabel("Observed equipment")).toHaveText("authorized equipment");
  await page.getByRole("button", { name: "Revoke observation", exact: true }).click();
  await expect(page.getByLabel("Observation phase")).toHaveText("denied");
  await expect(page.getByLabel("Observed equipment")).toHaveText("");
  await expect(page.getByLabel("Authority losses")).toHaveText("1");
});
