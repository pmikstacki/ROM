import { selectValue } from "./select-value.ts";
import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
test.beforeEach(async ({ page }) => {
  await page.goto("./tests/components/harness.html");
});
test("integer fields request a numeric mobile keyboard without losing exact input", async ({
  page,
}) => {
  await selectValue(page.getByLabel("count mode", { exact: true }), "value");
  const count = page.getByLabel("count value", { exact: true });
  await expect(count).toHaveAttribute("inputmode", "numeric");
  await count.fill("18446744073709551615");
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toContainText(
    "18446744073709551615",
  );
});
test("Resource references identify the target kind while accepting an exact ID", async ({
  page,
}) => {
  await selectValue(page.getByLabel("linked mode", { exact: true }), "value");
  const linked = page.getByLabel("linked value", { exact: true });
  await expect(linked).toHaveAttribute("type", "search");
  await expect(page.getByText("Resource ID in fixture")).toBeVisible();
  await linked.fill("row-2");
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toContainText('"value":"row-2"');
});
test("Resource controls preserve integer, presence, boolean and nested edits", async ({
  page,
}) => {
  await selectValue(page.getByLabel("count mode", { exact: true }), "value");
  await page
    .getByLabel("count value", { exact: true })
    .fill("18446744073709551615");
  await selectValue(page.getByLabel("done mode", { exact: true }), "value");
  await page.getByLabel("done value", { exact: true }).uncheck();
  await selectValue(page.getByLabel("note mode", { exact: true }), "null");
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
  await selectValue(page.getByLabel("code mode", { exact: true }), "value");
  await page.getByLabel("code custom value", { exact: true }).fill("ABC");
  await selectValue(page.getByLabel("tags mode", { exact: true }), "value");
  await page.getByRole("button", { name: "Add tags item" }).click();
  await page.getByLabel("tags[0] value", { exact: true }).fill("red");
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toContainText("ABC");
  await expect(page.getByTestId("submitted")).toContainText("red");
  await selectValue(page.getByLabel("amount mode", { exact: true }), "value");
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
  await selectValue(page.getByLabel("Query field"), "done");
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

test("descriptor change retains an open draft and requires explicit reopen", async ({
  page,
}) => {
  await selectValue(page.getByLabel("count mode", { exact: true }), "value");
  await page.getByLabel("count value", { exact: true }).fill("7");
  await selectValue(page.getByLabel("note mode", { exact: true }), "value");
  await page.getByLabel("note value", { exact: true }).fill("retained draft");
  await page
    .getByRole("button", { name: "Advance fixture descriptor" })
    .click();
  const form = page.locator("form").first();
  await expect(form.getByRole("alert")).toHaveText(
    "Resource definition changed. The draft is preserved. Reopen the form before submitting.",
  );
  await expect(page.getByLabel("count value", { exact: true })).toHaveValue(
    "7",
  );
  await expect(page.getByLabel("note value", { exact: true })).toHaveValue(
    "retained draft",
  );
  await expect(page.getByLabel("count value", { exact: true })).toBeDisabled();
  await expect(page.getByLabel("note value", { exact: true })).toBeDisabled();
  await expect(
    form.getByRole("button", { name: "Apply patch" }),
  ).toBeDisabled();
  await expect(page.getByTestId("submitted")).toHaveText("");
  const reopen = page.getByRole("button", {
    name: "Discard draft and reopen fixture",
  });
  await reopen.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByLabel("count mode", { exact: true })).toContainText(
    "Unchanged / omitted",
  );
  await expect(page.getByLabel("note mode", { exact: true })).toContainText(
    "Unchanged / omitted",
  );
  await expect(form.getByRole("alert")).toHaveCount(0);
  await selectValue(page.getByLabel("count mode", { exact: true }), "value");
  await page.getByLabel("count value", { exact: true }).fill("8");
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"count":{"op":"set","value":8}}}',
  );
});

test("invalid field keeps keyboard focus and cannot submit until corrected", async ({
  page,
}) => {
  await selectValue(page.getByLabel("count mode", { exact: true }), "value");
  const input = page.getByLabel("count value", { exact: true });
  await input.focus();
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.type("18446744073709551616");
  await expect(input).toBeFocused();
  await expect(input).toHaveAttribute("aria-invalid", "true");
  await expect(page.getByText("count: integer out of range")).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("submitted")).toHaveText("");
  await expect(input).toBeFocused();
  await expect(input).toHaveValue("18446744073709551616");
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.type("7");
  await expect(input).toBeFocused();
  await expect(input).toHaveAttribute("aria-invalid", "false");
  await expect(page.getByText("count: integer out of range")).toHaveCount(0);
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"count":{"op":"set","value":7}}}',
  );
});

test("shared Select Checkbox and submit keyboard semantics preserve a boolean", async ({
  page,
}) => {
  const mode = page.getByLabel("done mode", { exact: true });
  await mode.focus();
  await page.keyboard.press("Enter");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await page.keyboard.press("Tab");
  const checkbox = page.getByLabel("done value", { exact: true });
  await expect(mode).toContainText("Set value");
  await expect(checkbox).toBeFocused();
  await page.keyboard.press("Space");
  await expect(checkbox).toBeChecked();
  await page.getByRole("button", { name: "Apply patch" }).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"done":{"op":"set","value":true}}}',
  );
});

test("optional remove and empty string remain distinct from omitted fields", async ({
  page,
}) => {
  await selectValue(page.getByLabel("note mode", { exact: true }), "remove");
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"note":{"op":"remove"}}}',
  );
  await selectValue(page.getByLabel("note mode", { exact: true }), "value");
  await page.getByLabel("note value", { exact: true }).fill("");
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"note":{"op":"set","value":""}}}',
  );
  await selectValue(page.getByLabel("note mode", { exact: true }), "omit");
  await page.getByRole("button", { name: "Apply patch" }).click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{}}',
  );
});

test("nested invalid input survives sibling edits and removal releases only its error", async ({
  page,
}) => {
  await selectValue(page.getByLabel("scores mode", { exact: true }), "value");
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
  await selectValue(page.getByLabel("phase mode", { exact: true }), "value");
  await selectValue(page.getByLabel("phase value", { exact: true }), "closed");
  await selectValue(page.getByLabel("linked mode", { exact: true }), "value");
  await page.getByLabel("linked value", { exact: true }).fill("row-2");
  await selectValue(page.getByLabel("labels mode", { exact: true }), "value");
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
  await selectValue(page.getByLabel("Query field"), "done");
  await page.getByLabel("Query value value").check();
  await selectValue(page.getByLabel("Query sort field"), "count");
  await selectValue(page.getByLabel("Query sort", { exact: true }), "desc");
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
  await picker.click();
  await expect(
    page.getByRole("option", { name: "tags", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("option", { name: "labels", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("option", { name: "count", exact: true }),
  ).toHaveCount(1);
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
  await selectValue(
    page.getByLabel("maybe_code mode", { exact: true }),
    "value",
  );
  await page
    .locator("form")
    .first()
    .getByLabel("maybe_code custom value", { exact: true })
    .fill("OPTION");
  await selectValue(page.getByLabel("codes mode", { exact: true }), "value");
  await page
    .getByRole("button", { name: "Add codes item", exact: true })
    .click();
  await page.getByLabel("codes[0] custom value", { exact: true }).fill("LIST");
  await selectValue(page.getByLabel("code_map mode", { exact: true }), "value");
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
  await selectValue(
    page.getByLabel("maybe_code mode", { exact: true }),
    "null",
  );
  await page.getByRole("button", { name: "Apply patch", exact: true }).click();
  await expect(page.getByTestId("submitted")).toContainText(
    '"maybe_code":{"op":"set","value":null}',
  );
});
test("query filters preserve wrapper provenance and edit only registered leaves", async ({
  page,
}) => {
  await selectValue(page.getByLabel("Query field"), "codes");
  await page
    .getByRole("button", { name: "Add Query value item", exact: true })
    .click();
  await page
    .getByLabel("Query value[0] custom value", { exact: true })
    .fill("FILTER");
  await page.getByRole("button", { name: "Apply query", exact: true }).click();
  await expect(page.getByTestId("query-submitted")).toContainText(
    '"value":["FILTER"]',
  );
});
for (const count of [100, 500])
  test(`finite generic table ${count} row frame and DOM retention sample`, async ({
    page,
  }, testInfo) => {
    await page.goto(`./tests/components/harness.html?benchmark_rows=${count}`);
    const table = page.getByRole("table");
    await expect(
      table.getByRole("button", { name: `Open row-${count}`, exact: true }),
    ).toBeVisible();
    const visibleFrame = await page.evaluate(
      () =>
        new Promise<number>((resolve) =>
          requestAnimationFrame(() => resolve(performance.now())),
        ),
    );
    const columns = await table.getByRole("columnheader").count();
    const retainedBefore = await table.getByRole("cell").count();
    expect(retainedBefore).toBe(count * columns);
    const clearing = await page.evaluate(() => performance.now());
    await page
      .getByRole("button", { name: "Clear benchmark rows", exact: true })
      .click();
    await expect(table.getByRole("cell")).toHaveCount(0);
    const clearedFrame = await page.evaluate(
      () =>
        new Promise<number>((resolve) =>
          requestAnimationFrame(() => resolve(performance.now())),
        ),
    );
    const report = {
      engine: testInfo.project.name,
      rows: count,
      columns,
      cells_before: retainedBefore,
      cells_after_clear: 0,
      navigation_to_visible_frame_ms: visibleFrame,
      clear_to_frame_ms: clearedFrame - clearing,
      api_requests: 0,
      scope:
        "Local descriptor-only browser fixture, warm asset cache, real generic ResourceTable and custom codec renderers. Navigation includes module execution and frame scheduling. DOM retention count, not heap reclamation or database/parser cost.",
    };
    console.log("ROM_TABLE_FRAME " + JSON.stringify(report));
  });
