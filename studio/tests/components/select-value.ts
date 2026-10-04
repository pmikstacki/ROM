import { expect, type Locator } from "@playwright/test";
/** Exercise the active popup on the foreground page, preserving the exact value. */
export async function selectValue(control: Locator, value: string) {
  const page = control.page();
  await page.bringToFront();
  await control.click();
  const popup = page.locator('[role="listbox"][data-state="open"]');
  await expect(popup).toHaveCount(1);
  const escaped = value.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
  await popup
    .locator(`[role="option"][data-rom-select-value="${escaped}"]`)
    .click();
}
