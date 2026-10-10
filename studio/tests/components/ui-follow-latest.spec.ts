import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
const source = readFileSync(
  new URL(import.meta.resolve("rom-ui/ui/helpers/follow-latest")), "utf8",
).replace(/^export /gm, "");
async function setup(page: import("@playwright/test").Page) {
  await page.setContent(
    '<div id="feed" style="height:200px;overflow:auto;overflow-anchor:none"><div id="items" style="height:1000px">History</div></div>',
  );
  await page.addScriptTag({
    content:
      source +
      `\nwindow.followStates = []; const feed = document.getElementById('feed'); feed.scrollTop=800; window.follow = attachFollowLatest(feed,{thresholdPx:20,reducedMotion:true,onFollowing:value=>window.followStates.push(value)});`,
  });
  await page.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );
}
test("follow-latest preserves manual reading and supports explicit resume", async ({
  page,
}) => {
  await setup(page);
  await page.evaluate(() => {
    const feed = document.getElementById("feed")!;
    feed.scrollTop = 300;
    feed.dispatchEvent(new Event("scroll"));
    document.getElementById("items")!.style.height = "1400px";
    (window as any).follow.notifyContent();
  });
  await expect
    .poll(() => page.locator("#feed").evaluate((element) => element.scrollTop))
    .toBe(300);
  await page.evaluate(() => (window as any).follow.resume());
  await expect
    .poll(() => page.locator("#feed").evaluate((element) => element.scrollTop))
    .toBe(1200);
  expect(await page.evaluate(() => (window as any).follow.snapshot())).toEqual({
    following: true,
  });
  expect(await page.evaluate(() => (window as any).followStates)).toEqual([
    false,
    true,
  ]);
});
test("follow-latest uses actual frame and resized viewport without losing ownership", async ({
  page,
}) => {
  await setup(page);
  await page.evaluate(() => {
    document.getElementById("items")!.style.height = "1400px";
    (window as any).follow.notifyContent();
    (window as any).follow.notifyContent();
  });
  await expect
    .poll(() => page.locator("#feed").evaluate((element) => element.scrollTop))
    .toBe(1200);
  await page.locator("#feed").evaluate((element) => {
    (element as HTMLElement).style.height = "100px";
  });
  await expect
    .poll(() => page.locator("#feed").evaluate((element) => element.scrollTop))
    .toBe(1300);
  expect(await page.evaluate(() => (window as any).follow.snapshot())).toEqual({
    following: true,
  });
});
test("disposal suppresses a queued automatic scroll in the real browser", async ({
  page,
}) => {
  await setup(page);
  await page.evaluate(() => {
    document.getElementById("items")!.style.height = "1400px";
    (window as any).follow.notifyContent();
    (window as any).follow.dispose();
  });
  await page.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );
  expect(
    await page.locator("#feed").evaluate((element) => element.scrollTop),
  ).toBe(800);
});

test("small manual upward movement requires explicit resume", async ({
  page,
}) => {
  await setup(page);
  await page.locator("#feed").evaluate((element) => {
    element.scrollTop = 790;
    element.dispatchEvent(new Event("scroll"));
  });
  expect(await page.evaluate(() => (window as any).follow.snapshot())).toEqual({
    following: false,
  });
  await page.evaluate(() => {
    document.getElementById("items")!.style.height = "1400px";
    (window as any).follow.notifyContent();
  });
  await page.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );
  expect(
    await page.locator("#feed").evaluate((element) => element.scrollTop),
  ).toBe(790);
  await page.locator("#feed").evaluate((element) => {
    element.scrollTop = 1200;
    element.dispatchEvent(new Event("scroll"));
  });
  expect(await page.evaluate(() => (window as any).follow.snapshot())).toEqual({
    following: false,
  });
});

test("a larger viewport can clamp the position without disabling following", async ({
  page,
}) => {
  await setup(page);
  await page.locator("#feed").evaluate((element) => {
    (element as HTMLElement).style.height = "400px";
  });
  await expect
    .poll(() => page.locator("#feed").evaluate((element) => element.scrollTop))
    .toBe(600);
  expect(await page.evaluate(() => (window as any).follow.snapshot())).toEqual({
    following: true,
  });
});
