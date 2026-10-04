import { selectValue } from "../components/select-value.ts";
import { expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
export async function humanLogin(page: Page, url: string) {
  await page.goto(url);
  await page.getByRole("link", { name: "Sign in with Local fixture" }).click();
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await page.getByRole("button", { name: "Allow", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Sign out", exact: true }),
  ).toBeVisible();
}
export async function accessible(page: Page, stage: string) {
  await page.bringToFront();
  await page.evaluate(() =>
    Promise.race([
      Promise.allSettled(
        document.getAnimations().map((animation) => animation.finished),
      ),
      new Promise((resolve) => setTimeout(resolve, 500)),
    ]),
  );
  const result = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();
  expect(result.violations, stage).toEqual([]);
}
export async function createResource(page: Page, kind: string, id: string) {
  await page.getByRole("button", { name: kind, exact: true }).click();
  const trigger = page.getByRole("button", {
    name: "Create Resource",
    exact: true,
  });
  await expect(trigger).toBeEnabled();
  await trigger.focus();
  await expect(trigger).toBeFocused();
  await page.keyboard.press("Enter");
  const form = page.getByRole("region", {
    name: "Create Resource",
    exact: true,
  });
  await form.getByLabel("Resource ID", { exact: true }).fill(id);
  const field = kind === "tasks" ? "title" : "code";
  await selectValue(form.getByLabel(`${field} mode`, { exact: true }), "value");
  await form
    .getByLabel(`${field} value`, { exact: true })
    .fill(kind === "tasks" ? "Browser task" : "BROWSER-STOCK");
  const numeric = kind === "tasks" ? "done" : "quantity";
  await selectValue(
    form.getByLabel(`${numeric} mode`, { exact: true }),
    "value",
  );
  if (kind === "tasks")
    await form.getByLabel("done value", { exact: true }).uncheck();
  else
    await form
      .getByLabel("quantity value", { exact: true })
      .fill("18446744073709551615");
  await accessible(page, `${kind} create form`);
  await form
    .getByRole("button", { name: "Create resource", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: `Open ${id}`, exact: true }),
  ).toBeVisible();
}
export async function updateResource(
  page: Page,
  id: string,
  field: string,
  value: string,
) {
  await page.getByRole("button", { name: `Open ${id}`, exact: true }).click();
  const details = page.getByRole("region", {
    name: "Resource details",
    exact: true,
  });
  const input = details.getByLabel(`${field} value`, { exact: true });
  await input.focus();
  await expect(input).toBeFocused();
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.type(value);
  await details
    .getByRole("button", { name: "Save 1 change", exact: true })
    .click();
  await expect(page.getByText(value, { exact: true }).first()).toBeVisible();
}
