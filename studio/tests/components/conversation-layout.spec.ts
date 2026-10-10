import { test, expect } from "@playwright/test";
const fixture = "tests/components/conversation-layout.html";

test("leaving expansion focus does not reclaim focus after a viewport change", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(fixture);
  const expand = page.getByRole("button", {
    name: "Expand conversation",
    exact: true,
  });
  await expand.focus();
  await expand.evaluate((element) => element.blur());
  await page.setViewportSize({ width: 390, height: 500 });
  await page.waitForTimeout(100);
  await expect(
    page.getByRole("region", { name: "Conversation", exact: true }),
  ).not.toBeFocused();
});

test("keyboard users can focus and scroll noninteractive conversation content", async ({
  page,
}) => {
  await page.goto(fixture);
  const body = page.locator("[data-rom-conversation-body]");
  await body.evaluate((element) => {
    element.scrollTop = 0;
  });
  await body.focus();
  await expect(body).toBeFocused();
  await page.keyboard.press("PageDown");
  await expect
    .poll(() => body.evaluate((element) => element.scrollTop))
    .toBeGreaterThan(0);
});

test("hiding the focused expansion control keeps focus in the conversation", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(fixture);
  await page
    .getByRole("button", { name: "Expand conversation", exact: true })
    .focus();
  await page.setViewportSize({ width: 390, height: 500 });
  await expect(
    page.getByRole("region", { name: "Conversation", exact: true }),
  ).toBeFocused();
  await expect(
    page.getByRole("textbox", { name: "Message", exact: true }),
  ).toBeInViewport({ ratio: 1 });
});

for (const viewport of [
  { width: 1280, height: 900 },
  { width: 390, height: 844 },
  { width: 390, height: 500 },
]) {
  test(`composer stays reachable with long messages and history at ${viewport.width}x${viewport.height}`, async ({
    page,
  }) => {
    await page.setViewportSize(viewport);
    await page.goto(fixture);
    await expect(
      page.getByRole("region", { name: "Conversation", exact: true }),
    ).toBeVisible();
    await page.getByText("Saved history", { exact: true }).click();
    await expect(
      page.getByRole("textbox", { name: "Message", exact: true }),
    ).toBeInViewport({ ratio: 1 });
    await expect(
      page.getByRole("button", { name: "Send message", exact: true }),
    ).toBeInViewport({ ratio: 1 });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    if (viewport.width < 800)
      await expect(
        page.getByRole("button", { name: "Expand conversation", exact: true }),
      ).toBeHidden();
  });
}

test("desktop expansion and viewport changes preserve raw draft node and focus", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(fixture);
  const region = page.getByRole("region", {
    name: "Conversation",
    exact: true,
  });
  const draft = page.getByRole("textbox", { name: "Message", exact: true });
  await draft.fill("raw draft 1e+");
  await draft.evaluate((element) => {
    element.setAttribute("data-mount", "original");
  });
  const before = (await region.boundingBox())!.width;
  const expand = page.getByRole("button", {
    name: "Expand conversation",
    exact: true,
  });
  await expand.focus();
  await page.keyboard.press("Space");
  await expect(
    page.getByRole("button", { name: "Collapse conversation", exact: true }),
  ).toBeFocused();
  await expect
    .poll(async () => (await region.boundingBox())!.width)
    .toBeGreaterThan(before);
  await draft.focus();
  await page.setViewportSize({ width: 390, height: 500 });
  await expect(draft).toBeFocused();
  await expect(draft).toHaveValue("raw draft 1e+");
  await expect(draft).toHaveAttribute("data-mount", "original");
  await page.setViewportSize({ width: 1280, height: 900 });
  await expect(draft).toBeFocused();
  await expect(draft).toHaveAttribute("data-mount", "original");
});

test("caller composer handles keyboard and IME without layout owning submission", async ({
  page,
}) => {
  await page.goto(fixture);
  const draft = page.getByRole("textbox", { name: "Message", exact: true });
  await draft.fill("hello");
  await draft.press("Shift+Enter");
  await expect(draft).toHaveValue("hello\n");
  await draft.dispatchEvent("keydown", { key: "Enter", isComposing: true });
  await expect(page.getByLabel("Submitted messages")).toHaveText("0");
  await draft.press("Enter");
  await expect(page.getByLabel("Submitted messages")).toHaveText("1");
  await draft.fill("button path");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(page.getByLabel("Submitted messages")).toHaveText("2");
});

test("caller follow-latest preserves manual scrolling and resumes explicitly", async ({
  page,
}) => {
  await page.goto(fixture);
  const body = page.locator("[data-rom-conversation-body]");
  await expect
    .poll(() => body.evaluate((element) => element.scrollTop))
    .toBeGreaterThan(0);
  await body.evaluate((element) => {
    element.scrollTop = 100;
    element.dispatchEvent(new Event("scroll"));
  });
  await expect(page.getByLabel("Following latest")).toHaveText("no");
  await page
    .getByRole("button", { name: "Append message", exact: true })
    .click();
  await expect
    .poll(() => body.evaluate((element) => element.scrollTop))
    .toBe(100);
  await page
    .getByRole("button", { name: "Resume latest", exact: true })
    .click();
  await expect
    .poll(() =>
      body.evaluate(
        (element) =>
          element.scrollHeight - element.clientHeight - element.scrollTop,
      ),
    )
    .toBeLessThan(1);
  await expect(page.getByLabel("Following latest")).toHaveText("yes");
});

test("layout remains reachable under reduced motion", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 390, height: 500 });
  await page.goto(fixture);
  await expect(
    page.getByRole("textbox", { name: "Message", exact: true }),
  ).toBeInViewport({ ratio: 1 });
  expect(
    await page
      .getByRole("region", { name: "Conversation", exact: true })
      .evaluate((element) => getComputedStyle(element).animationName),
  ).toBe("none");
});
