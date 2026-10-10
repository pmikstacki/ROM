import { test, expect } from "@playwright/test";

test("public ResourceForm preserves exact integers and decimals in a patch", async ({ page }) => {
  await page.goto("/?forms");
  await page.getByLabel("amount value", { exact: true }).fill("9007199254740993.000000000000000002");
  await page.getByLabel("count value", { exact: true }).fill("9007199254740995");
  await page.getByRole("button", { name: /^Save 2 changes$/ }).click();
  const submitted = page.getByLabel("Submitted form", { exact: true });
  await expect(submitted).toContainText('"9007199254740993.000000000000000002"');
  await expect(submitted).toContainText('9007199254740995');
  await expect(submitted).not.toContainText('9007199254740996');
});

test("public form keeps the candidate after application rejection", async ({ page }) => {
  await page.goto("/?forms");
  await page.getByLabel("Reject application submission", { exact: true }).check();
  const field = page.getByLabel("amount value", { exact: true });
  await field.fill("27.125");
  await page.getByRole("button", { name: "Save 1 change", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Application rejected this patch.");
  await expect(field).toHaveValue("27.125");
  await expect(page.getByLabel("Submitted form", { exact: true })).toHaveText("none");
});
