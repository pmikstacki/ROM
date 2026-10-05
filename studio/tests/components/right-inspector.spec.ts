import { test, expect } from "@playwright/test";

test("right inspector toggles by keyboard and preserves filter drafts", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("tests/components/details.html");
  const toggle = page.getByRole("button", {
    name: "Close details panel",
    exact: true,
  });
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  const target = await toggle.getAttribute("aria-controls");
  expect(target).toBeTruthy();
  const pane = page.locator(`[id="${target}"]`);
  await expect(pane).toBeVisible();
  const paneBox = await pane.boundingBox();
  const tableBox = await page.getByRole("table").boundingBox();
  expect(paneBox!.x).toBeGreaterThan(tableBox!.x + tableBox!.width);
  await page.getByRole("tab", { name: "Filters", exact: true }).click();
  const editor = page.getByRole("region", {
    name: "Filter editor",
    exact: true,
  });
  const input = editor.getByRole("textbox", {
    name: "Filter 1 value",
    exact: true,
  });
  await input.fill("unapplied draft");
  await toggle.focus();
  await page.keyboard.press("Enter");
  const closed = page.getByRole("button", {
    name: "Open details panel",
    exact: true,
  });
  await expect(closed).toBeFocused();
  await expect(closed).toHaveAttribute("aria-expanded", "false");
  await expect(pane).toBeHidden();
  await page.keyboard.press("Space");
  await expect(input).toHaveValue("unapplied draft");
  await expect(
    editor.getByText("Pending edits", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("group", { name: "Applied query", exact: true }),
  ).toContainText("original");
  await expect(
    page.getByRole("button", { name: "Close details panel", exact: true }),
  ).toBeFocused();
});

test("right mobile Sheet restores toggle focus and preserves Resource draft across resize", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("tests/components/details.html");
  const title = page.getByRole("textbox", { name: "title value", exact: true });
  await title.fill("draft survives");
  await page.setViewportSize({ width: 390, height: 844 });
  const sheet = page.getByRole("dialog", {
    name: "Edit Resource",
    exact: true,
  });
  await expect(sheet).toBeVisible();
  await expect(sheet).toHaveAttribute("data-side", "right");
  await expect(title).toHaveValue("draft survives");
  await page.keyboard.press("Tab");
  expect(
    await page
      .locator(":focus")
      .evaluate((node) => !!node.closest('[data-slot="sheet-content"]')),
  ).toBe(true);
  await page.keyboard.press("Escape");
  const toggle = page.getByRole("button", {
    name: "Open details panel",
    exact: true,
  });
  await expect(toggle).toBeFocused();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await page.keyboard.press("Enter");
  await expect(sheet).toBeVisible();
  const target = await page
    .getByRole("button", { name: "Close details panel", exact: true })
    .getAttribute("aria-controls");
  await expect(sheet).toHaveAttribute("id", target!);
  await expect(title).toHaveValue("draft survives");
  await page.setViewportSize({ width: 1440, height: 1000 });
  await expect(title).toHaveValue("draft survives");
  await expect(
    page.getByRole("complementary", { name: "Resource tools", exact: true }),
  ).toHaveAttribute("id", target!);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});
