import { test, expect } from "@playwright/test";

test("installed public workspace composes cards, history, references and layout controls", async ({ page }) => {
  await page.goto("/?ui");
  const workspace = page.getByRole("region", { name: "Installed public workspace compositions" });
  const card = workspace.getByRole("button", { name: "Feed pump", exact: true });
  await card.focus();
  await page.keyboard.press("Space");
  await expect(card).toHaveAttribute("aria-pressed", "true");
  await workspace.getByLabel("Related equipment value", { exact: true }).fill("valve-1");
  await expect(workspace.getByLabel("Related equipment value", { exact: true })).toHaveValue("valve-1");
  await workspace.getByRole("button", { name: "Feed pump inspection", exact: true }).click();
  await expect(workspace.getByLabel("Selected inspection", { exact: true })).toHaveText("inspection-1");
  await workspace.getByRole("button", { name: "Move right Inspection history", exact: true }).click();
  await expect(workspace.getByLabel("Confirmed history position", { exact: true })).toHaveText("1");
  await workspace.getByRole("button", { name: "Review selected equipment", exact: true }).click();
  await expect(page.getByRole("region", { name: "Equipment inspection", exact: true })).toBeVisible();
});

test("installed public workspace remains usable without horizontal overflow at 390 pixels", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/?ui");
  const workspace = page.getByRole("region", { name: "Installed public workspace compositions" });
  await expect(workspace.getByRole("button", { name: "Feed pump", exact: true })).toBeVisible();
  const geometry = await workspace.evaluate((element) => ({ width: element.clientWidth, scroll: element.scrollWidth }));
  expect(geometry.scroll).toBeLessThanOrEqual(geometry.width + 1);
  await workspace.getByRole("button", { name: "Move right Inspection history", exact: true }).click();
  await expect(workspace.getByLabel("Confirmed history position", { exact: true })).toHaveText("1");
});
