import { test, expect } from "@playwright/test";
const fixture = "tests/components/reference-composition.html";
const trigger = "Choose Assigned person reference";
test("selection callback authority rebind never associates the old label with the new owner", async ({
  page,
}) => {
  await page.goto(fixture);
  await page.getByRole("button", { name: "Revoke during selection" }).click();
  await page.getByRole("button", { name: trigger }).click();
  await page.getByRole("option", { name: /Owner private label/ }).click();
  await expect(
    page.getByText("Owner private label", { exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("searchbox", { name: "Assigned person value" }),
  ).toHaveValue("  α/exact?key#id  ");
});
test("host labels update while pending and readonly stops the pending request", async ({
  page,
}) => {
  await page.goto(fixture);
  await page.getByRole("button", { name: "Slow response" }).click();
  await page.getByRole("button", { name: trigger }).click();
  await expect(page.getByLabel("Lookup count")).toHaveText("1");
  await page
    .getByRole("button", { name: "Use Polish labels" })
    .evaluate((button) => button.click());
  await expect(
    page.getByRole("combobox", { name: "Szukaj osób" }),
  ).toBeVisible();
  await expect(page.locator("p[role=status]")).toHaveText("Wczytywanie osób…");
  await page
    .getByRole("button", { name: "Make readonly" })
    .evaluate((button) => button.click());
  await expect(
    page.getByRole("searchbox", { name: "Assigned person value" }),
  ).toBeDisabled();
  await expect(page.getByLabel("Aborted count")).toHaveText("1");
  await page.waitForTimeout(700);
  await expect(
    page.getByText("Owner private label", { exact: true }),
  ).toHaveCount(0);
});
test("public picker selects with keyboard and preserves exact ID", async ({
  page,
}) => {
  await page.goto(fixture);
  await page.getByRole("button", { name: trigger }).click();
  await expect(
    page.getByText("Owner private label", { exact: true }),
  ).toBeVisible();
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("searchbox", { name: "Assigned person value" }),
  ).toHaveValue("  α/exact?key#id  ");
  await page.getByRole("button", { name: "Change authority" }).click();
  await expect(
    page.getByText("Owner private label", { exact: true }),
  ).toHaveCount(0);
});
test("authority change aborts and fences uncooperative late response", async ({
  page,
}) => {
  await page.goto(fixture);
  await page.getByRole("button", { name: "Slow response" }).click();
  await page.getByRole("button", { name: trigger }).click();
  await expect(page.getByLabel("Lookup count")).toHaveText("1");
  await page
    .getByRole("button", { name: "Change authority" })
    .evaluate((button) => button.click());
  await expect(
    page.getByRole("combobox", { name: "Search Assigned person candidates" }),
  ).toHaveCount(0);
  await page.waitForTimeout(700);
  await expect(
    page.getByText("Owner private label", { exact: true }),
  ).toHaveCount(0);
  await expect(page.getByLabel("Aborted count")).toHaveText("1");
});
for (const mode of ["Deny response", "Overflow response"]) {
  test(`${mode} leaves manual exact ID entry available`, async ({ page }) => {
    await page.goto(fixture);
    await page.getByRole("button", { name: mode }).click();
    await page.getByRole("button", { name: trigger }).click();
    await expect(page.locator("p[role=status]")).not.toHaveText(
      "Loading authorized candidates…",
    );
    await expect(page.getByRole("option")).toHaveCount(0);
    await page.keyboard.press("Escape");
    await page
      .getByRole("searchbox", { name: "Assigned person value" })
      .fill("  manual/α?exact#id  ");
    await expect(page.getByLabel("Exact ID")).toHaveText(
      JSON.stringify("  manual/α?exact#id  "),
    );
  });
}
test("nonempty search debounces and caps an opened flow at eight attempts", async ({
  page,
}) => {
  await page.goto(fixture);
  await page.getByRole("button", { name: trigger }).click();
  await expect(page.getByLabel("Lookup count")).toHaveText("1");
  const search = page.getByRole("combobox", {
    name: "Search Assigned person candidates",
  });
  await search.fill("a");
  await page.waitForTimeout(80);
  await search.fill("ab");
  await page.waitForTimeout(80);
  await expect(page.getByLabel("Lookup count")).toHaveText("1");
  await expect(page.getByLabel("Lookup count")).toHaveText("2");
  for (let index = 3; index <= 8; index++) {
    await search.fill(String(index));
    await expect(page.getByLabel("Lookup count")).toHaveText(String(index));
  }
  await search.fill("ninth");
  await expect(page.locator("p[role=status]")).toContainText(
    "search limit reached",
  );
  await expect(page.getByLabel("Lookup count")).toHaveText("8");
});
