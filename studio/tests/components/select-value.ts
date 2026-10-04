import type { Locator } from "@playwright/test";
/** Exercise the public popup control; preserve the exact domain option value. */
export async function selectValue(control: Locator, value: string) {
  await control.click();
  const escaped = value.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
  await control
    .page()
    .locator(`[role="option"][data-rom-select-value="${escaped}"]`)
    .click();
}
