import { test, expect } from "@playwright/test";
const longTitle = "Inspection ".repeat(45);
const fixture = "tests/components/history-composition.html";

test("history titles stay readable and have full accessible names on narrow screens", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 500 });
  await page.goto(fixture);
  await expect(
    page.getByRole("button", { name: longTitle.trim(), exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText('opaque-["local","human","alice"]', { exact: true }),
  ).toHaveCount(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await expect(page.locator('button[aria-current="true"]')).toHaveCount(1);
});

test("native keyboard activation calls the host and moves only its confirmed current marker", async ({
  page,
}) => {
  await page.goto(fixture);
  const row = page.getByRole("button", {
    name: "Maintenance summary",
    exact: true,
  });
  await row.focus();
  await page.keyboard.press("Space");
  await expect(row).toHaveAttribute("aria-current", "true");
  await expect(page.getByLabel("Navigation attempts")).toHaveText("1");
});

test("history metadata and empty text follow a locale change", async ({
  page,
}) => {
  await page.goto(fixture);
  const before = await page.locator("time").first().innerText();
  await page.getByRole("button", { name: "Use Polish", exact: true }).click();
  await expect(page.locator("time").first()).not.toHaveText(before);
  await expect(page.getByText(/1\s?000 wpisów/)).toBeVisible();
  await page.getByRole("button", { name: "Toggle empty", exact: true }).click();
  await expect(page.getByText("Brak historii", { exact: true })).toBeVisible();
});

test("uncertain writes and refused host navigation cannot optimistically change history", async ({
  page,
}) => {
  await page.goto(fixture);
  const row = page.getByRole("button", {
    name: "Maintenance summary",
    exact: true,
  });
  await page
    .getByRole("button", { name: "Refuse navigation", exact: true })
    .click();
  await row.click();
  await expect(row).not.toHaveAttribute("aria-current", "true");
  await page
    .getByRole("button", { name: "Uncertain mutation", exact: true })
    .click();
  await expect(row).toBeDisabled();
  await expect(page.getByLabel("Navigation attempts")).toHaveText("1");
});

test("pending failure is sanitized and its message reacts to locale changes", async ({
  page,
}) => {
  await page.goto(fixture);
  await page
    .getByRole("button", { name: "Hold navigation", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Maintenance summary", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Maintenance summary", exact: true }),
  ).toBeDisabled();
  await page
    .getByRole("button", { name: "Fail navigation", exact: true })
    .click();
  await expect(
    page.getByText("Cannot open entry", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("private exception", { exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Use Polish", exact: true }).click();
  await expect(
    page.getByText("Nie można otworzyć wpisu", { exact: true }),
  ).toBeVisible();
});

test("old-owner pending navigation cannot report a failure in the new scope", async ({
  page,
}) => {
  await page.goto(fixture);
  await page
    .getByRole("button", { name: "Hold navigation", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Maintenance summary", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Change authority", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Fail navigation", exact: true })
    .click();
  await expect(
    page.getByText("Cannot open entry", { exact: true }),
  ).toHaveCount(0);
  await expect(page.locator('button[aria-current="true"]')).toHaveCount(0);
});

test("NaN authority tokens do not strand pending navigation", async ({
  page,
}) => {
  await page.goto(fixture);
  await page
    .getByRole("button", { name: "Use NaN token", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Hold navigation", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Maintenance summary", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Fail navigation", exact: true })
    .click();
  await expect(
    page.getByText("Cannot open entry", { exact: true }),
  ).toBeVisible({ timeout: 2000 });
  await expect(
    page.getByRole("button", { name: "Maintenance summary", exact: true }),
  ).toBeEnabled();
});
