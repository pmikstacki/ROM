import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("./tests/components/harness.html");
});

test("shared mode picker uses keyboard and keeps nullable operation controlled", async ({
  page,
}) => {
  const mode = page.getByRole("button", { name: "note mode", exact: true });
  await mode.focus();
  await page.keyboard.press("Enter");
  await expect(mode).toHaveAttribute("aria-expanded", "true");
  await page.getByRole("option", { name: "Set null", exact: true }).click();
  await expect(mode).toHaveAttribute("title", "Set null");
  await expect(page.getByLabel("note value", { exact: true })).toHaveCount(0);
  await mode.focus();
  await page.keyboard.press("Enter");
  await page.getByRole("option", { name: "Set value", exact: true }).click();
  await page.getByLabel("note value", { exact: true }).fill("kept draft");
  await expect(page.getByLabel("note value", { exact: true })).toHaveValue(
    "kept draft",
  );
});

test("shared query picker preserves independent sort selection and closes on Escape", async ({
  page,
}) => {
  const filter = page.getByRole("button", {
    name: "Query field",
    exact: true,
  });
  await filter.click();
  await expect(filter).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("Escape");
  await expect(filter).toBeFocused();
  await expect(filter).toHaveAttribute("aria-expanded", "false");
  const sort = page.getByRole("button", {
    name: "Query sort field",
    exact: true,
  });
  await sort.click();
  await page.getByRole("option", { name: "count", exact: true }).click();
  await expect(sort).toContainText("count");
  await expect(filter).toContainText("No filter");
});
