import { test, expect, type Page } from "@playwright/test";

const viewports = [{ width: 1280, height: 900 }, { width: 390, height: 844 }, { width: 390, height: 500 }];
async function openDetails(page: Page) {
  await page.goto("/?layout-acceptance");
  await page.getByRole("button", { name: "Open conversation details", exact: true }).click();
  const frame = page.getByRole("region", { name: "Combined conversation", exact: true });
  await expect(frame).toBeVisible();
  return frame;
}

async function settleLayout(frame: import("@playwright/test").Locator) {
  await frame.evaluate(async element => {
    await Promise.race([Promise.all(document.getAnimations().map(animation => animation.finished.catch(() => undefined))), new Promise((_, reject) => setTimeout(() => reject(new Error("Layout animation deadline")), 2000))]);
    let previous = element.getBoundingClientRect(), stable = 0;
    for (let sample = 0; sample < 120; sample++) {
      await new Promise(requestAnimationFrame);
      const next = element.getBoundingClientRect();
      stable = Math.abs(next.left - previous.left) < .1 && Math.abs(next.right - previous.right) < .1 && document.getAnimations().every(animation => animation.playState !== "running") ? stable + 1 : 0;
      previous = next;
      if (stable === 3) return;
    }
    throw new Error("Stable layout deadline");
  });
}

for (const viewport of viewports) {
  test(`public combined history and composer are bounded at ${viewport.width}x${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    const frame = await openDetails(page);
    const draft = frame.getByLabel("Message draft", { exact: true });
    await draft.fill("Draft retained while scrolling\n".repeat(12));
    await settleLayout(frame);
    const geometry = await frame.evaluate((element) => {
      const rect = element.getBoundingClientRect();
      const textarea = element.querySelector("textarea")!.getBoundingClientRect();
      const send = element.querySelector('button[type="submit"]')!.getBoundingClientRect();
      const body = element.querySelector('[data-rom-conversation-body]')!;
      const history = element.querySelector('ul[aria-label="Saved conversations"]')!.parentElement!;
      return { left: rect.left, right: rect.right, bottom: rect.bottom, viewportWidth: innerWidth, viewportHeight: innerHeight, draft: { top: textarea.top, bottom: textarea.bottom }, send: { top: send.top, bottom: send.bottom }, bodyHeight: body.clientHeight, bodyScrollable: body.scrollHeight > body.clientHeight, historyScrollable: history.parentElement!.scrollHeight > history.parentElement!.clientHeight };
    });
    expect(geometry.left).toBeGreaterThanOrEqual(0);
    expect(geometry.right).toBeLessThanOrEqual(geometry.viewportWidth + 1);
    expect(geometry.bottom).toBeLessThanOrEqual(geometry.viewportHeight + 1);
    expect(geometry.bodyHeight).toBeGreaterThan(0);
    expect(geometry.bodyScrollable).toBe(true);
    expect(geometry.historyScrollable).toBe(true);
    expect(geometry.draft.top).toBeGreaterThanOrEqual(0);
    expect(geometry.send.bottom).toBeLessThanOrEqual(geometry.viewportHeight + 1);
    await expect(draft).toBeVisible();
    await frame.getByRole("button", { name: "Send message", exact: true }).click();
    await expect(frame.getByLabel("Submitted messages")).toHaveText("1");
    await expect(draft).toHaveValue("");
  });
  test(`public empty history and authority change refuse stale publication at ${viewport.width}x${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    const frame = await openDetails(page);
    await frame.getByRole("button", { name: "Hold next history", exact: true }).click();
    await frame.getByRole("button", { name: /^Saved conversation 1:/ }).click();
    await expect(frame.getByLabel("History request phase")).toHaveText("pending");
    await expect(frame.getByLabel("Pending history identity")).toHaveText("0:history-0-0");
    await frame.getByRole("button", { name: "Change authority", exact: true }).click();
    await expect(frame.getByLabel("Selected history")).toHaveText("none");
    await expect(frame.getByLabel("Pending history identity")).toHaveText("none");
    await frame.getByRole("button", { name: "Complete held history", exact: true }).click();
    await expect(frame.getByLabel("Selected history")).toHaveText("none");
    await expect(frame.getByLabel("History request phase")).toHaveText("idle");
    await expect(frame.getByText("History selection failed", { exact: true })).toHaveCount(0);
    await frame.getByRole("button", { name: /^Saved conversation 1:/ }).click();
    await expect(frame.getByLabel("Selected history")).toHaveText("history-1-0");
    await frame.getByRole("button", { name: "Toggle empty history", exact: true }).click();
    await expect(frame.getByText("No saved conversations", { exact: true })).toBeVisible();
    await expect(frame.getByRole("list", { name: "Saved conversations" })).toHaveCount(0);
    await expect(frame.getByLabel("Message draft", { exact: true })).toBeVisible();
  });
}

test("public combined expansion and breakpoint preserve the exact editor, draft, selection and pending request", async ({ page }) => {
  await page.setViewportSize(viewports[0]);
  const frame = await openDetails(page);
  const draft = frame.getByLabel("Message draft", { exact: true });
  await draft.fill("Exact unchanged multiline draft\nSecond line");
  await draft.evaluate((element: HTMLTextAreaElement) => { element.dataset.identity = "same-editor"; element.setSelectionRange(2, 9); });
  await frame.getByRole("button", { name: /^Saved conversation 2:/ }).click();
  await expect(frame.getByLabel("Selected history")).toHaveText("history-0-1");
  await frame.getByRole("button", { name: "Hold next history", exact: true }).click();
  await frame.getByRole("button", { name: /^Saved conversation 1:/ }).click();
  const before = await frame.boundingBox();
  await frame.getByRole("button", { name: "Expand conversation", exact: true }).click();
  expect((await frame.boundingBox())!.width).toBeGreaterThan(before!.width);
  await expect(frame.getByLabel("Pending history identity")).toHaveText("0:history-0-0");
  await expect(frame.getByLabel("Selected history")).toHaveText("history-0-1");
  await expect(draft).toHaveAttribute("data-identity", "same-editor");
  await draft.focus();
  await draft.evaluate((element: HTMLTextAreaElement) => element.setSelectionRange(2, 9));
  await page.setViewportSize(viewports[2]);
  await expect(page.getByRole("dialog", { name: "Conversation details", exact: true })).toBeVisible();
  await expect(draft).toBeFocused();
  await expect(draft).toHaveValue("Exact unchanged multiline draft\nSecond line");
  await expect(draft).toHaveAttribute("data-identity", "same-editor");
  expect(await draft.evaluate((element: HTMLTextAreaElement) => [element.selectionStart, element.selectionEnd])).toEqual([2, 9]);
  await expect(frame.getByLabel("Pending history identity")).toHaveText("0:history-0-0");
  await expect(frame.getByLabel("Selected history")).toHaveText("history-0-1");
  await page.setViewportSize(viewports[0]);
  await expect(page.getByRole("dialog", { name: "Conversation details", exact: true })).toHaveCount(0);
  await expect(draft).toHaveAttribute("data-identity", "same-editor");
  await expect(draft).toBeFocused();
  expect(await draft.evaluate((element: HTMLTextAreaElement) => [element.selectionStart, element.selectionEnd])).toEqual([2, 9]);
  await draft.dispatchEvent("keydown", { key: "Enter", isComposing: true });
  await expect(frame.getByLabel("Submitted messages")).toHaveText("0");
  await expect(draft).toHaveValue("Exact unchanged multiline draft\nSecond line");
  await expect(frame.getByLabel("Pending history identity")).toHaveText("0:history-0-0");
  await frame.getByRole("button", { name: "Complete held history", exact: true }).click();
  await expect(frame.getByLabel("Selected history")).toHaveText("history-0-0");
});

test("public compact details Escape returns focus and restores scroll lock through breakpoint changes", async ({ page }) => {
  await page.setViewportSize(viewports[1]);
  await page.goto("/?layout-acceptance");
  const initial = await page.evaluate(() => ({ overflow: document.body.style.overflow, padding: document.body.style.paddingRight }));
  const opener = page.getByRole("button", { name: "Open conversation details", exact: true });
  await opener.click();
  await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe("hidden");
  await page.keyboard.press("Escape");
  await expect(opener).toBeFocused();
  await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe(initial.overflow);
  expect(await page.evaluate(() => document.body.style.paddingRight)).toBe(initial.padding);
  await opener.click();
  await page.setViewportSize(viewports[0]);
  await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe(initial.overflow);
  await page.setViewportSize(viewports[2]);
  await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe("hidden");
  await page.keyboard.press("Escape");
  await expect(opener).toBeFocused();
  await expect.poll(() => page.evaluate(() => document.body.style.overflow)).toBe(initial.overflow);
});
