import { test, expect, type Page } from "@playwright/test";
import { selectValue } from "./select-value.ts";
const fixture = "tests/components/sortable-fields.html";
async function open(page: Page) {
  await page.goto(fixture);
  await page.getByRole("button", { name: "Edit entries", exact: true }).click();
  return page.getByRole("dialog", { name: "entries", exact: true });
}

test("manual reorder keeps invalid duplicate row draft and blocks resource submission", async ({
  page,
}) => {
  const editor = await open(page);
  await editor.getByLabel("entries[1] value", { exact: true }).fill("invalid");
  await editor
    .getByRole("button", { name: "Move entries[1] down", exact: true })
    .click();
  const moved = editor.getByLabel("entries[2] value", { exact: true });
  await expect(moved).toHaveValue("invalid");
  await expect(moved).toHaveAttribute("aria-invalid", "true");
  await expect(moved).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("button", { name: "Save 1 change", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Edit entries", exact: true }).click();
  await page
    .getByRole("dialog", { name: "entries", exact: true })
    .getByLabel("entries[2] value", { exact: true })
    .fill("11");
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: "Save 1 change", exact: true })
    .click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"entries":{"op":"set","value":[7,9,11]}}}',
  );
});

test("keyboard drag submits only canonical values and retains duplicate identity", async ({
  page,
}) => {
  const editor = await open(page);
  await editor.getByLabel("entries[1] value", { exact: true }).fill("8");
  await editor
    .getByRole("button", { name: "Drag entries[1]", exact: true })
    .focus();
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Space");
  await expect(
    editor.getByLabel("entries[2] value", { exact: true }),
  ).toHaveValue("8");
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: "Save 1 change", exact: true })
    .click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"entries":{"op":"set","value":[7,9,8]}}}',
  );
});

test("pointer reorder keeps invalid drafts attached and removal clears row error", async ({
  page,
}) => {
  const editor = await open(page);
  await editor.getByLabel("entries[1] value", { exact: true }).fill("invalid");
  const a = await editor
    .getByRole("button", { name: "Drag entries[1]", exact: true })
    .boundingBox();
  const z = await editor
    .getByText("Item 3", { exact: true })
    .locator("..")
    .boundingBox();
  expect(a && z).toBeTruthy();
  await page.mouse.move(a!.x + a!.width / 2, a!.y + a!.height / 2);
  await page.mouse.down();
  await page.mouse.move(a!.x + a!.width / 2, z!.y + z!.height - 5, {
    steps: 20,
  });
  await page.waitForTimeout(300);
  await page.mouse.up();
  await expect(
    editor.getByLabel("entries[2] value", { exact: true }),
  ).toHaveValue("invalid");
  await editor
    .getByRole("button", { name: "Remove entries[2]", exact: true })
    .click();
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: "Save 1 change", exact: true })
    .click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"entries":{"op":"set","value":[7,9]}}}',
  );
});

test("readonly and bounded lists expose no drag or mutation controls", async ({
  page,
}) => {
  await page.goto(fixture);
  for (const name of ["Readonly list", "Bounded list"]) {
    const area = page.getByRole("region", { name, exact: true });
    await expect(area.getByRole("button", { name: /Drag|Move/ })).toHaveCount(
      0,
    );
    for (const control of await area
      .getByRole("button", { name: /Add|Remove/ })
      .all())
      await expect(control).toBeDisabled();
  }
});

test("codec row draft survives pointer replacement without losing exact decimal source", async ({
  page,
}) => {
  await page.goto(fixture);
  await page.getByRole("button", { name: "Edit amounts", exact: true }).click();
  const editor = page.getByRole("dialog", { name: "amounts", exact: true });
  await editor.getByLabel("amounts[1] value", { exact: true }).fill("invalid");
  const a = await editor
    .getByRole("button", { name: "Drag amounts[1]", exact: true })
    .boundingBox();
  const z = await editor
    .getByText("Item 3", { exact: true })
    .locator("..")
    .boundingBox();
  expect(a && z).toBeTruthy();
  await page.mouse.move(a!.x + a!.width / 2, a!.y + a!.height / 2);
  await page.mouse.down();
  await page.mouse.move(a!.x + a!.width / 2, z!.y + z!.height - 5, {
    steps: 20,
  });
  await page.waitForTimeout(300);
  await page.mouse.up();
  await expect(
    editor.getByLabel("amounts[2] value", { exact: true }),
  ).toHaveValue("invalid");
  await expect(
    editor.getByLabel("amounts[2] value", { exact: true }),
  ).toHaveAttribute("aria-invalid", "true");
  await editor.getByLabel("amounts[2] value", { exact: true }).fill("1.2300");
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: "Save 1 change", exact: true })
    .click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"amounts":{"op":"set","value":["1.2300","9.0000","1.23"]}}}',
  );
});

test("action inputs use the same canonical list reorder", async ({ page }) => {
  await page.goto(fixture);
  const area = page.getByRole("region", { name: "Action editor", exact: true });
  await selectValue(
    area.getByRole("button", { name: "entries mode", exact: true }),
    "value",
  );
  await area.getByRole("button", { name: "Edit entries", exact: true }).click();
  const editor = page.getByRole("dialog", { name: "entries", exact: true });
  for (let i = 0; i < 3; i++)
    await editor
      .getByRole("button", { name: "Add entries item", exact: true })
      .click();
  for (const [i, text] of ["7", "7", "9"].entries())
    await editor
      .getByLabel("entries[" + i + "] value", { exact: true })
      .fill(text);
  await editor
    .getByRole("button", { name: "Move entries[2] up", exact: true })
    .click();
  await page.keyboard.press("Escape");
  await area.getByRole("button", { name: "Run reorder", exact: true }).click();
  await expect(page.getByTestId("invoked")).toHaveText('{"entries":[7,9,7]}');
});

test("filter candidates retain invalid row state after reorder", async ({
  page,
}) => {
  await page.goto(fixture);
  const area = page.getByRole("region", { name: "Filter editor", exact: true });
  await area.getByLabel("Filter 1[1] value", { exact: true }).fill("invalid");
  await area
    .getByRole("button", { name: "Move Filter 1[1] down", exact: true })
    .click();
  await expect(
    area.getByLabel("Filter 1[2] value", { exact: true }),
  ).toHaveValue("invalid");
  await expect(
    area.getByRole("button", { name: "Apply filters", exact: true }),
  ).toBeDisabled();
  await area.getByLabel("Filter 1[2] value", { exact: true }).fill("11");
  await area
    .getByRole("button", { name: "Apply filters", exact: true })
    .click();
  const query = JSON.parse((await page.getByTestId("filtered").textContent())!);
  expect(query.filters).toEqual([{ field: "entries", value: [7, 9, 11] }]);
});

test("touch handle reorders a duplicate row without serializing drag metadata", async ({
  page,
  context,
  browserName,
}) => {
  test.skip(
    browserName !== "chromium",
    "CDP touch input is Chromium-specific; WebKit physical touch remains an explicit acceptance gap.",
  );
  const editor = await open(page);
  const a = await editor
    .getByRole("button", { name: "Drag entries[1]", exact: true })
    .boundingBox();
  const z = await editor
    .getByText("Item 3", { exact: true })
    .locator("..")
    .boundingBox();
  expect(a && z).toBeTruthy();
  const session = await context.newCDPSession(page);
  const x = a!.x + a!.width / 2,
    y = a!.y + a!.height / 2,
    target = z!.y + z!.height - 5;
  await session.send("Input.dispatchTouchEvent", {
    type: "touchStart",
    touchPoints: [{ x, y }],
  });
  await page.waitForTimeout(150);
  for (let i = 1; i <= 12; i++) {
    await session.send("Input.dispatchTouchEvent", {
      type: "touchMove",
      touchPoints: [{ x, y: y + ((target - y) * i) / 12 }],
    });
    await page.waitForTimeout(35);
  }
  await page.waitForTimeout(300);
  await session.send("Input.dispatchTouchEvent", {
    type: "touchEnd",
    touchPoints: [],
  });
  await session.detach();
  await expect(
    editor.getByLabel("entries[1] value", { exact: true }),
  ).toHaveValue("9");
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: "Save 1 change", exact: true })
    .click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"entries":{"op":"set","value":[7,9,7]}}}',
  );
});

test("closing a keyboard drag releases the action and preserves the form candidate", async ({
  page,
}) => {
  const editor = await open(page);
  await editor
    .getByRole("button", { name: "Drag entries[1]", exact: true })
    .focus();
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Escape");
  if (await editor.isVisible()) await page.keyboard.press("Escape");
  await expect(editor).not.toBeVisible();
  await page.getByRole("button", { name: "Edit entries", exact: true }).click();
  const reopened = page.getByRole("dialog", { name: "entries", exact: true });
  await reopened
    .getByRole("button", { name: "Move entries[2] up", exact: true })
    .click();
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: "Save 1 change", exact: true })
    .click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"entries":{"op":"set","value":[7,7,9]}}}',
  );
  await expect(page.locator("#dnd-action-dragged-el")).toHaveCount(0);
});

test("prototype-shaped map keys keep invalid drafts local to their editor", async ({
  page,
}) => {
  await page.goto(fixture);
  await page
    .getByRole("button", { name: "Edit dictionary", exact: true })
    .click();
  const editor = page.getByRole("dialog", { name: "dictionary", exact: true });
  await editor
    .getByLabel("dictionary.__proto__ value", { exact: true })
    .fill("invalid");
  expect(
    await page.evaluate(() => Object.hasOwn(Object.prototype, "text")),
  ).toBe(false);
  await expect(
    editor.getByLabel("dictionary.__proto__ value", { exact: true }),
  ).toHaveAttribute("aria-invalid", "true");
  await expect(
    editor.getByLabel("dictionary.constructor value", { exact: true }),
  ).toHaveValue("3");
  await editor
    .getByLabel("dictionary.__proto__ value", { exact: true })
    .fill("4");
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: "Save 1 change", exact: true })
    .click();
  const patch = JSON.parse(
    (await page.getByTestId("submitted").textContent())!,
  );
  expect(patch.input.dictionary.value).toEqual(
    JSON.parse('{"__proto__":4,"constructor":3}'),
  );
});

test("labelled enum bulk append preserves duplicates and canonical ordered values", async ({
  page,
}) => {
  await page.goto(fixture);
  const resource = page.getByRole("region", { name: "Resource editor" });
  await resource
    .getByRole("button", { name: "Edit choices", exact: true })
    .click();
  const editor = page.getByRole("dialog", { name: "choices", exact: true });
  await expect(
    editor.getByLabel("choices[0] value", { exact: true }),
  ).toContainText("Active");
  await editor
    .getByRole("button", { name: "Choose choices items", exact: true })
    .click();
  await page
    .getByRole("searchbox", { name: "Search choices choices" })
    .fill("Active");
  await page
    .getByRole("checkbox", {
      name: "Select Active (paused) for choices",
      exact: true,
    })
    .click();
  await page
    .getByRole("checkbox", {
      name: "Select Active (running) for choices",
      exact: true,
    })
    .click();
  await page
    .getByRole("button", { name: "Append selected choices items", exact: true })
    .click();
  await expect(editor.getByRole("listitem")).toHaveCount(4);
  await page.keyboard.press("Escape");
  await resource
    .getByRole("button", { name: "Save 1 change", exact: true })
    .click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"choices":{"op":"set","value":["running","running","running","paused"]}}}',
  );
});

test("bulk chooser cancellation emits nothing and unknown stored enum remains visible", async ({
  page,
}) => {
  await page.goto(fixture);
  const resource = page.getByRole("region", { name: "Resource editor" });
  await resource
    .getByRole("button", { name: "Edit choices", exact: true })
    .click();
  const editor = page.getByRole("dialog", { name: "choices", exact: true });
  await editor
    .getByRole("button", { name: "Choose choices items", exact: true })
    .click();
  await page
    .getByRole("checkbox", {
      name: "Select Waiting (queued) for choices",
      exact: true,
    })
    .click();
  await page.keyboard.press("Escape");
  await expect(editor.getByRole("listitem")).toHaveCount(2);
  await editor
    .getByRole("button", { name: "Choose choices items", exact: true })
    .click();
  await expect(
    page.getByRole("checkbox", {
      name: "Select Waiting (queued) for choices",
      exact: true,
    }),
  ).not.toBeChecked();
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await expect(
    resource.getByRole("button", { name: "Save changes", exact: true }),
  ).toBeDisabled();
  const unknown = page.getByRole("region", { name: "Unknown enum" });
  await expect(unknown.getByRole("alert")).toContainText("retired");
  await unknown.getByLabel("note value", { exact: true }).fill("changed");
  await expect(
    unknown.getByRole("button", { name: "Save 1 change", exact: true }),
  ).toBeDisabled();
  await selectValue(
    unknown.getByLabel("phase value", { exact: true }),
    "queued",
  );
  await expect(
    unknown.getByRole("button", { name: "Save 2 changes", exact: true }),
  ).toBeEnabled();
});

test("action and filter reuse labelled enum bulk append with exact members", async ({
  page,
}) => {
  await page.goto(fixture);
  const action = page.getByRole("region", { name: "Enum action editor" });
  await selectValue(
    action.getByRole("button", { name: "choices mode", exact: true }),
    "value",
  );
  await action
    .getByRole("button", { name: "Edit choices", exact: true })
    .click();
  let editor = page.getByRole("dialog", { name: "choices", exact: true });
  await editor
    .getByRole("button", { name: "Choose choices items", exact: true })
    .click();
  await page
    .getByRole("checkbox", {
      name: "Select Active (paused) for choices",
      exact: true,
    })
    .click();
  await page
    .getByRole("button", { name: "Append selected choices items", exact: true })
    .click();
  await page.keyboard.press("Escape");
  await action.getByRole("button", { name: "Run choose", exact: true }).click();
  await expect(page.getByTestId("enum-invoked")).toHaveText(
    '{"choices":["paused"]}',
  );
  const filters = page.getByRole("region", { name: "Filter editor" });
  await selectValue(
    filters.getByLabel("Filter 1 field", { exact: true }),
    "choices",
  );
  await filters
    .getByRole("button", { name: "Choose Filter 1 items", exact: true })
    .click();
  await page
    .getByRole("checkbox", {
      name: "Select Waiting (queued) for Filter 1",
      exact: true,
    })
    .click();
  await page
    .getByRole("button", {
      name: "Append selected Filter 1 items",
      exact: true,
    })
    .click();
  await filters
    .getByRole("button", { name: "Apply filters", exact: true })
    .click();
  await expect(page.getByTestId("filtered")).toContainText(
    '"value":["queued"]',
  );
});

test("enum labels display across wrapped collections and unknown members stay exact", async ({
  page,
}) => {
  await page.goto(fixture);
  await expect(
    page.getByRole("region", { name: "Nested enum display" }),
  ).toHaveText("[{first: Waiting,second: Active,unknown: retired}]");
  const resource = page.getByRole("region", { name: "Resource editor" });
  await expect(
    resource.getByLabel("phase value", { exact: true }),
  ).toContainText("Waiting");
  await selectValue(
    resource.getByLabel("phase value", { exact: true }),
    "paused",
  );
  await resource
    .getByRole("button", { name: "Save 1 change", exact: true })
    .click();
  await expect(page.getByTestId("submitted")).toHaveText(
    '{"type":"patch","input":{"phase":{"op":"set","value":"paused"}}}',
  );
});

test("renderer unregister lifecycle preserves newer inline registration and collections stay expanded", async ({
  page,
}) => {
  await page.goto(fixture);
  await expect(page.getByTestId("registration-retained")).toHaveText("true");
  await expect(page.getByTestId("invalid-layout")).toHaveText("true");
  const area = page.getByRole("region", { name: "Registry lifecycle" });
  await expect(
    area.getByLabel("scalar registered value", { exact: true }),
  ).toHaveValue("inline");
  await expect(
    area.getByRole("button", { name: "Edit scalar", exact: true }),
  ).toHaveCount(0);
  await expect(
    area.getByRole("button", { name: "Edit collection", exact: true }),
  ).toBeVisible();
  await expect(
    area.getByLabel("collection[0] registered value", { exact: true }),
  ).toHaveCount(0);
  await area
    .getByRole("button", { name: "Edit collection", exact: true })
    .click();
  await expect(
    page
      .getByRole("dialog", { name: "collection", exact: true })
      .getByLabel("collection[0] registered value", { exact: true }),
  ).toHaveValue("expanded");
});

for (const [field, input, bad] of [
  ["entries", "entries[1] value", "invalid-number"],
  ["amounts", "amounts[1] value", "invalid-decimal"],
  ["dictionary", "dictionary.__proto__ value", "invalid-map"],
]) {
  test(`closing and reopening ${field} retains invalid draft after dialog unmount`, async ({
    page,
  }) => {
    await page.goto(fixture);
    const area = page.getByRole("region", {
      name: "Resource editor",
      exact: true,
    });
    await selectValue(
      area.getByLabel("phase value", { exact: true }),
      "paused",
    );
    await area
      .getByRole("button", { name: `Edit ${field}`, exact: true })
      .click();
    let editor = page.getByRole("dialog", { name: field, exact: true });
    await editor.getByLabel(input, { exact: true }).fill(bad);
    await expect(editor.getByLabel(input, { exact: true })).toHaveAttribute(
      "aria-invalid",
      "true",
    );
    await page.keyboard.press("Escape");
    await expect(editor).toHaveCount(0);
    await expect(
      area.getByRole("button", { name: "Save 1 change", exact: true }),
    ).toBeDisabled();
    await area
      .getByRole("button", { name: `Edit ${field}`, exact: true })
      .click();
    editor = page.getByRole("dialog", { name: field, exact: true });
    await expect(editor.getByLabel(input, { exact: true })).toHaveValue(bad);
    await expect(editor.getByLabel(input, { exact: true })).toHaveAttribute(
      "aria-invalid",
      "true",
    );
    await page.keyboard.press("Escape");
    await expect(editor).toHaveCount(0);
    await expect(
      area.getByRole("button", { name: "Save 1 change", exact: true }),
    ).toBeDisabled();
  });
}
