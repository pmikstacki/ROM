import { test, expect, type Page } from "@playwright/test";

const fixture = "tests/components/renderer-regressions.html";

for (const [name, text] of [
  ["Scalar text", "Visible scalar"],
  ["Scalar zero", "0"],
  ["Scalar false", "false"],
  ["Explicit null", "Null"],
  ["Missing optional", "Absent"],
  ["Empty codec list", "[]"],
  ["Empty codec map", "{}"],
]) {
  test(`generic display exposes visible text for ${name}`, async ({ page }) => {
    await page.goto(fixture);
    // Target ValueDisplay's text element, not the enclosing table cell. A cell
    // can have height while its inline text element has a zero-height box.
    const value = page
      .getByRole("region", { name, exact: true })
      .getByText(text, { exact: true });
    await expect(value).toHaveCount(1);
    await expect(value).toBeVisible();
  });
}

async function openMobile(page: Page) {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(`${fixture}?mobile`);
  await page
    .getByRole("button", { name: "Open resource tools", exact: true })
    .click();
  const sheet = page.getByRole("dialog", {
    name: "Resource tools",
    exact: true,
  });
  await expect(sheet).toBeVisible();
  return sheet;
}

test("mobile nested Select remains usable after scrolling its Sheet", async ({
  page,
}) => {
  const sheet = await openMobile(page);
  const direction = sheet.getByRole("button", {
    name: "Direction",
    exact: true,
  });
  await direction.click();
  const descending = page.getByRole("option", {
    name: "Descending",
    exact: true,
  });
  await expect(descending).toBeVisible();
  const box = await sheet.boundingBox();
  expect(box).not.toBeNull();
  if (!box) throw new Error("Visible Sheet has no box");
  // Use the Sheet's left padding below the popup, away from the portaled menu.
  await page.mouse.move(box.x + 8, box.y + box.height - 80);
  await page.mouse.wheel(0, 80);
  await expect
    .poll(() => sheet.evaluate((node) => node.scrollTop))
    .toBeGreaterThan(0);
  await expect(descending).toBeVisible();
  await descending.click();
  await expect(direction).toContainText("Descending");
  await expect(page.getByRole("listbox")).not.toBeVisible();
  await sheet
    .getByRole("button", { name: "Apply direction", exact: true })
    .click();
  await expect(
    sheet.getByLabel("Applied direction", { exact: true }),
  ).toHaveText("desc");
  await expect(sheet).toBeVisible();
});

test("mobile nested Select dismisses by ordinary outside click without closing its Sheet", async ({
  page,
}) => {
  const sheet = await openMobile(page);
  const direction = sheet.getByRole("button", {
    name: "Direction",
    exact: true,
  });
  const apply = sheet.getByRole("button", {
    name: "Apply direction",
    exact: true,
  });
  await direction.click();
  await expect(page.getByRole("listbox")).toBeVisible();
  // This paragraph is above the portaled listbox, so the click is truly outside.
  await sheet
    .getByText("Choose a direction, then apply the draft.", { exact: true })
    .click();
  await expect(page.getByRole("listbox")).not.toBeVisible();
  await expect(sheet).toBeVisible();
  await expect(direction).toContainText("Ascending");
  await apply.click();
  await expect(
    sheet.getByLabel("Applied direction", { exact: true }),
  ).toHaveText("asc");
});

test("mobile Escape dismisses Select before Sheet and restores toolbar focus", async ({
  page,
}) => {
  const sheet = await openMobile(page);
  const direction = sheet.getByRole("button", {
    name: "Direction",
    exact: true,
  });
  await direction.click();
  await expect(page.getByRole("listbox")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("listbox")).not.toBeVisible();
  await expect(sheet).toBeVisible();
  await expect(direction).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(
    sheet.getByRole("button", { name: "Apply direction", exact: true }),
  ).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(sheet).not.toBeVisible();
  await expect(
    page.getByRole("button", { name: "Open resource tools", exact: true }),
  ).toBeFocused();
});
