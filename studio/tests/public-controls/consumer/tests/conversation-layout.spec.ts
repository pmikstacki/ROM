import { test, expect } from "@playwright/test";

for (const viewport of [{ width: 1280, height: 900 }, { width: 390, height: 500 }]) {
  test(`installed conversation bounds history and composer at ${viewport.width}x${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto("/?ui");
    const frame = page.getByRole("region", { name: "Inspection conversation", exact: true });
    await frame.scrollIntoViewIfNeeded();
    const history = frame.locator("summary");
    await history.focus();
    await page.keyboard.press("Space");
    await expect(frame.locator("details")).toHaveAttribute("open", "");
    const draft = frame.getByLabel("Conversation message", { exact: true });
    await draft.fill("A multiline draft\n".repeat(25));
    const geometry = await frame.evaluate((element) => {
      const rect = element.getBoundingClientRect();
      const textarea = element.querySelector("textarea")!.getBoundingClientRect();
      const body = element.querySelector('[role="region"]')!;
      const bodyRect = body.getBoundingClientRect();
      return { width: rect.width, viewport: innerWidth, draftBottom: textarea.bottom, frameBottom: rect.bottom, bodyHeight: bodyRect.height, scrollable: body.scrollHeight > body.clientHeight };
    });
    expect(geometry.width).toBeLessThanOrEqual(geometry.viewport);
    expect(geometry.draftBottom).toBeLessThanOrEqual(geometry.frameBottom);
    expect(geometry.bodyHeight).toBeGreaterThan(0);
    expect(geometry.scrollable).toBe(true);
    await expect(draft).toBeVisible();
  });
}

test("installed conversation expansion preserves snippet identity and responsive draft focus", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto("/?ui");
  const frame = page.getByRole("region", { name: "Inspection conversation", exact: true });
  const draft = frame.getByLabel("Conversation message", { exact: true });
  await draft.fill("Raw installed draft");
  await draft.evaluate((element) => { element.dataset.identity = "original"; });
  const initial = await frame.boundingBox();
  const expand = frame.getByRole("button", { name: "Expand inspection conversation", exact: true });
  await expand.focus();
  await page.keyboard.press("Space");
  await expect(frame.getByRole("button", { name: "Collapse inspection conversation", exact: true })).toHaveAttribute("aria-pressed", "true");
  expect((await frame.boundingBox())!.width).toBeGreaterThan(initial!.width);
  await draft.focus();
  await page.setViewportSize({ width: 390, height: 500 });
  await expect(draft).toBeFocused();
  await expect(draft).toHaveValue("Raw installed draft");
  await expect(draft).toHaveAttribute("data-identity", "original");
});

test("installed conversation composer keeps Shift Enter and submits ordinary Enter", async ({ page }) => {
  await page.goto("/?ui");
  const draft = page.getByLabel("Conversation message", { exact: true });
  await draft.fill("First line");
  await draft.press("End");
  await draft.press("Shift+Enter");
  await expect(draft).toHaveValue("First line\n");
  await expect(page.getByLabel("Conversation submissions")).toHaveText("0");
  await draft.dispatchEvent("keydown", { key: "Enter", isComposing: true });
  await expect(page.getByLabel("Conversation submissions")).toHaveText("0");
  await draft.press("Enter");
  await expect(page.getByLabel("Conversation submissions")).toHaveText("1");
  await expect(draft).toHaveValue("");
});

test("installed conversation named message region supports keyboard scrolling", async ({ page }) => {
  await page.goto("/?ui");
  const body = page.getByRole("region", { name: "Inspection messages", exact: true });
  await body.focus();
  await expect(body).toBeFocused();
  const before = await body.evaluate((element) => element.scrollTop);
  await page.keyboard.press("PageDown");
  await expect.poll(() => body.evaluate((element) => element.scrollTop)).toBeGreaterThan(before);
});

test("installed conversation transfers hidden expansion focus but respects explicit departure", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto("/?ui");
  const frame = page.getByRole("region", { name: "Inspection conversation", exact: true });
  const expand = frame.getByRole("button", { name: "Expand inspection conversation", exact: true });
  await expand.focus();
  await page.setViewportSize({ width: 390, height: 500 });
  await expect(frame).toBeFocused();
  await page.setViewportSize({ width: 1280, height: 900 });
  await expand.focus();
  await expand.evaluate((element) => element.blur());
  await page.setViewportSize({ width: 390, height: 500 });
  await expect(frame).not.toBeFocused();
});
