import { test, expect } from "@playwright/test";
const fixture = "tests/components/details-composition.html";

test("desktop details are a named nonmodal region with an independent focusable summary", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(fixture);
  await page.getByRole("button", { name: "Open inspection details", exact: true }).click();
  const region = page.getByRole("region", { name: "Inspection details", exact: true });
  await expect(region).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "Inspection summary", exact: true }).focus();
  await expect(page.getByRole("button", { name: "Inspection summary", exact: true })).toBeFocused();
  await page.getByRole("button", { name: "Close inspection details", exact: true }).click();
  await expect(region).toBeHidden();
  await expect(page.getByRole("button", { name: "Open inspection details", exact: true })).toBeFocused();
});

test("mobile details are modal, scroll long content, and Escape returns focus", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 500 });
  await page.goto(fixture);
  await page.getByRole("button", { name: "Open inspection details", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Inspection details", exact: true });
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveAttribute("aria-modal", "true");
  await dialog.getByRole("button", { name: "Final inspection action", exact: true }).scrollIntoViewIfNeeded();
  await expect(dialog.getByRole("button", { name: "Final inspection action", exact: true })).toBeInViewport();
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(page.getByRole("button", { name: "Open inspection details", exact: true })).toBeFocused();
});

test("one mounted editor retains its raw draft through desktop and mobile transitions", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(fixture);
  await page.getByRole("button", { name: "Open inspection details", exact: true }).click();
  await page.getByRole("textbox", { name: "Inspection draft" }).fill("later invalid raw draft 1e+");
  await page.getByRole("textbox", { name: "Inspection draft" }).evaluate(element => { element.setAttribute("data-mounted-witness", "original-node"); });
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole("dialog", { name: "Inspection details", exact: true })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Inspection draft" })).toHaveValue("later invalid raw draft 1e+");
  await expect(page.getByRole("textbox", { name: "Inspection draft" })).toHaveAttribute("data-mounted-witness", "original-node");
  await page.setViewportSize({ width: 1440, height: 900 });
  await expect(page.getByRole("region", { name: "Inspection details", exact: true })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Inspection draft" })).toHaveValue("later invalid raw draft 1e+");
  await expect(page.getByRole("textbox", { name: "Inspection draft" })).toHaveAttribute("data-mounted-witness", "original-node");
});

for (const action of ["Remove opening control", "Disable opening control"]) {
  test(`mobile close uses explicit fallback after ${action}`, async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(fixture);
    await page.getByRole("button", { name: "Open inspection details", exact: true }).click();
    await page.getByRole("button", { name: action, exact: true }).click();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("button", { name: "Inspection summary", exact: true })).toBeFocused();
  });
}

test("active close control uses the host's changed label", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(fixture);
  await page.getByRole("button", { name: "Open inspection details", exact: true }).click();
  await page.getByRole("button", { name: "Use Polish close label", exact: true }).click();
  await page.getByRole("button", { name: "Zamknij szczegóły inspekcji", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Inspection details", exact: true })).toBeHidden();
  await expect(page.getByRole("button", { name: "Open inspection details", exact: true })).toBeFocused();
});

test("closing one mobile details lifetime retains another modal's body scroll lock", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(fixture);
  await page.getByRole("button", { name: "Open inspection details", exact: true }).click();
  await page.getByRole("button", { name: "Open nested details", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Nested inspection details", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Close outer details", exact: true }).click();
  await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe("hidden");
  await page.keyboard.press("Escape");
  await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe("");
});

test("reduced motion displays complete details without content animation", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(fixture);
  await page.getByRole("button", { name: "Open inspection details", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Inspection details", exact: true });
  await expect(dialog).toBeVisible();
  const motion = await dialog.evaluate(element => ({ animation: getComputedStyle(element).animationName, transition: getComputedStyle(element).transitionDuration }));
  expect(motion).toEqual({ animation: "none", transition: "0s" });
});
