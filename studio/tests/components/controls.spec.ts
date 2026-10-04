import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
test.beforeEach(async ({ page }) => {
  await page.goto("./tests/components/harness.html");
});
test("Resource controls preserve integer, presence, boolean and nested edits", async ({
  page,
}) => {
  await page.getByLabel("count mode", { exact: true }).selectOption("value");
  await page
    .getByLabel("count value", { exact: true })
    .fill("18446744073709551615");
  await page.getByLabel("done mode", { exact: true }).selectOption("value");
  await page.getByLabel("done value", { exact: true }).uncheck();
  await page.getByLabel("note mode", { exact: true }).selectOption("null");
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toContainText(
    "18446744073709551615",
  );
  await expect(page.getByTestId("submitted")).toContainText(
    '"done":{"op":"set","value":false}',
  );
  await expect(page.getByTestId("submitted")).toContainText(
    '"note":{"op":"set","value":null}',
  );
  await page
    .getByLabel("count value", { exact: true })
    .fill("18446744073709551616");
  await expect(page.getByText("count: integer out of range")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Apply patch" }),
  ).toBeDisabled();
});
test("same field hosts support custom codec, nested collections and generic action input", async ({
  page,
}) => {
  await page.getByLabel("code mode", { exact: true }).selectOption("value");
  await page.getByLabel("code custom value", { exact: true }).fill("ABC");
  await page.getByLabel("tags mode", { exact: true }).selectOption("value");
  await page.getByRole("button", { name: "Add tags item" }).click();
  await page.getByLabel("tags[0] value", { exact: true }).fill("red");
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toContainText("ABC");
  await expect(page.getByTestId("submitted")).toContainText("red");
  await page.getByLabel("amount mode", { exact: true }).selectOption("value");
  await page
    .getByLabel("amount value", { exact: true })
    .fill("-9223372036854775808");
  await page.getByRole("button", { name: "Run adjust" }).click();
  await expect(page.getByTestId("action-submitted")).toContainText(
    "-9223372036854775808",
  );
});
test("unsupported action stays visible, generic table selects, query emits values", async ({
  page,
}) => {
  await expect(
    page.getByText(
      "This action has opaque inputs. A typed form is unavailable.",
    ),
  ).toBeVisible();
  await page.getByRole("button", { name: "Open row-1" }).click();
  await expect(page.getByTestId("selected")).toHaveText("row-1");
  await page.getByLabel("Query field").selectOption("done");
  await page.getByLabel("Query value value").check();
  await page.getByRole("button", { name: "Apply query" }).click();
  await expect(page.getByTestId("query-submitted")).toContainText(
    '"value":true',
  );
});
test("keyboard controls and assembled accessibility", async ({ page }) => {
  await page.getByRole("button", { name: "Open row-1" }).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("selected")).toHaveText("row-1");
  const scan = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();
  expect(scan.violations).toEqual([]);
});

test("optional remove and empty string remain distinct from omitted fields", async ({
  page,
}) => {
  await page.getByLabel("note mode", { exact: true }).selectOption("remove");
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"note":{"op":"remove"}}}',
  );
  await page.getByLabel("note mode", { exact: true }).selectOption("value");
  await page.getByLabel("note value", { exact: true }).fill("");
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"note":{"op":"set","value":""}}}',
  );
  await page.getByLabel("note mode", { exact: true }).selectOption("omit");
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{}}',
  );
});

test("nested invalid input survives sibling edits and removal releases only its error", async ({
  page,
}) => {
  await page.getByLabel("scores mode", { exact: true }).selectOption("value");
  await page.getByRole("button", { name: "Add scores item" }).click();
  await page.getByRole("button", { name: "Add scores item" }).click();
  await page
    .getByLabel("scores[0] value", { exact: true })
    .fill("9223372036854775808");
  await page.getByLabel("scores[1] value", { exact: true }).fill("1");
  await expect(
    page.getByRole("button", { name: "Apply patch" }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Remove scores[0]" }).click();
  await expect(page.getByRole("button", { name: "Apply patch" })).toBeEnabled();
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toContainText('"value":[1]');
});

test("enum reference and prototype-shaped map keys use generic controls", async ({
  page,
}) => {
  await page.getByLabel("phase mode", { exact: true }).selectOption("value");
  await page.getByLabel("phase value", { exact: true }).selectOption("closed");
  await page.getByLabel("linked mode", { exact: true }).selectOption("value");
  await page.getByLabel("linked value", { exact: true }).fill("row-2");
  await page.getByLabel("labels mode", { exact: true }).selectOption("value");
  await page.getByLabel("labels new key", { exact: true }).fill("__proto__");
  await page.getByRole("button", { name: "Add labels entry" }).click();
  await page.getByLabel("labels.__proto__ value", { exact: true }).fill("kept");
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toContainText(
    '"__proto__":"kept"',
  );
  await expect(page.getByTestId("submitted")).toContainText('"value":"closed"');
  await expect(page.getByTestId("submitted")).toContainText('"value":"row-2"');
});
test("opaque input uses the shared strict lossless codec", async ({ page }) => {
  await page
    .getByLabel("opaque raw JSON input")
    .fill('{"large":18446744073709551615}');
  await page.getByRole("button", { name: "Run opaque" }).click();
  await expect(page.getByTestId("opaque-submitted")).toHaveText(
    '{"large":18446744073709551615}',
  );
  await page.getByLabel("opaque raw JSON input").fill('{"large":1,"large":1}');
  await page.getByRole("button", { name: "Run opaque" }).click();
  await expect(page.getByRole("alert")).toContainText(/duplicate/i);
  await expect(page.getByTestId("opaque-submitted")).toHaveText(
    '{"large":18446744073709551615}',
  );
});

test("sort field is independent from filter field", async ({ page }) => {
  await page.getByLabel("Query field").selectOption("done");
  await page.getByLabel("Query value value").check();
  await page.getByLabel("Query sort field").selectOption("count");
  await page.getByLabel("Query sort", { exact: true }).selectOption("desc");
  await page.getByRole("button", { name: "Apply query" }).click();
  await expect(page.getByTestId("query-submitted")).toContainText(
    '"field":"count","direction":"desc"',
  );
  await expect(page.getByTestId("query-submitted")).toContainText(
    '"field":"done","value":true',
  );
});

test("sort picker excludes collection fields but retains scalar fields", async ({
  page,
}) => {
  const picker = page.getByLabel("Query sort field");
  await expect(picker.locator('option[value="tags"]')).toHaveCount(0);
  await expect(picker.locator('option[value="labels"]')).toHaveCount(0);
  await expect(picker.locator('option[value="count"]')).toHaveCount(1);
});
test("wrapped custom codecs apply only to leaves; unknown leaves remain read-only", async ({
  page,
}) => {
  await expect(
    page.locator("table").getByLabel("codes custom value", { exact: true }),
  ).toHaveValue("LEAF");
  await expect(
    page.locator("table").getByLabel("code_map custom value", { exact: true }),
  ).toHaveValue("ENTRY");
  await expect(
    page
      .locator("table")
      .getByLabel("maybe_code custom value", { exact: true }),
  ).toHaveValue("DETAIL");
  await expect(
    page.getByLabel("unknown_code mode", { exact: true }),
  ).toBeDisabled();
  await page
    .getByLabel("maybe_code mode", { exact: true })
    .selectOption("value");
  await page
    .locator("form")
    .first()
    .getByLabel("maybe_code custom value", { exact: true })
    .fill("OPTION");
  await page.getByLabel("codes mode", { exact: true }).selectOption("value");
  await page
    .getByRole("button", { name: "Add codes item", exact: true })
    .click();
  await page.getByLabel("codes[0] custom value", { exact: true }).fill("LIST");
  await page.getByLabel("code_map mode", { exact: true }).selectOption("value");
  await page.getByLabel("code_map new key", { exact: true }).fill("primary");
  await page
    .getByRole("button", { name: "Add code_map entry", exact: true })
    .click();
  await page
    .getByLabel("code_map.primary custom value", { exact: true })
    .fill("MAP");
  await page.getByRole("button", { name: "Apply patch", exact: true }).click();
  await expect(page.getByTestId("submitted")).toContainText(
    '"maybe_code":{"op":"set","value":"OPTION"}',
  );
  await expect(page.getByTestId("submitted")).toContainText(
    '"codes":{"op":"set","value":["LIST"]}',
  );
  await expect(page.getByTestId("submitted")).toContainText(
    '"code_map":{"op":"set","value":{"primary":"MAP"}}',
  );
  await expect(page.getByTestId("submitted")).not.toContainText("unknown_code");
  await page
    .getByLabel("maybe_code mode", { exact: true })
    .selectOption("null");
  await page.getByRole("button", { name: "Apply patch", exact: true }).click();
  await expect(page.getByTestId("submitted")).toContainText(
    '"maybe_code":{"op":"set","value":null}',
  );
});
test("query filters preserve wrapper provenance and edit only registered leaves", async ({ page }) => {
  await page.getByLabel("Query field").selectOption("codes");
  await page.getByRole("button", { name: "Add Query value item", exact: true }).click();
  await page.getByLabel("Query value[0] custom value", { exact: true }).fill("FILTER");
  await page.getByRole("button", { name: "Apply query", exact: true }).click();
  await expect(page.getByTestId("query-submitted")).toContainText('"value":["FILTER"]');
});
