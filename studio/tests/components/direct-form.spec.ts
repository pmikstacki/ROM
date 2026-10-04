import { test, expect } from "@playwright/test";

test("current null and absent fields are distinct from an empty value", async ({
  page,
}) => {
  await page.goto("tests/components/direct-form.html");
  await expect(page.getByText("Current value: null")).toHaveCount(1);
  await expect(page.getByText("Current value: not set")).toHaveCount(1);
  await expect(
    page.getByRole("button", { name: "Save changes" }),
  ).toBeDisabled();
});

test("generic scalar fields fit label, control and actions in one mobile row", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("tests/components/direct-form.html");
  const row = page.getByRole("group", { name: "title", exact: true });
  const label = await row.getByText("title", { exact: true }).boundingBox();
  const control = await row.getByLabel("title value").boundingBox();
  const actions = await row.getByLabel("title options").boundingBox();
  expect(label && control && actions).toBeTruthy();
  expect(
    Math.abs(label!.y + label!.height / 2 - (control!.y + control!.height / 2)),
  ).toBeLessThan(12);
  expect(
    Math.abs(
      actions!.y + actions!.height / 2 - (control!.y + control!.height / 2),
    ),
  ).toBeLessThan(12);
  expect((await row.boundingBox())!.height).toBeLessThan(70);
});

test("choosing Set a value starts from the current value", async ({ page }) => {
  await page.goto("tests/components/direct-form.html");
  await page.getByLabel("title options").click();
  await page.getByRole("menuitem", { name: "Set a value" }).click();
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "original",
  );
  await page.getByRole("button", { name: "Save 1 change" }).click();
  expect(
    JSON.parse(
      (await page.getByTestId("direct-submitted").textContent()) ?? "",
    ),
  ).toEqual({
    type: "patch",
    input: { title: { op: "set", value: "original" } },
  });
});

test("switching an invalid omitted field to a value resets its draft", async ({
  page,
}) => {
  await page.goto("tests/components/direct-form.html");
  const count = page.getByLabel("count value", { exact: true });
  await count.fill("invalid");
  await expect(count).toHaveAttribute("aria-invalid", "true");
  await page.getByLabel("count options").click();
  await page.getByRole("menuitem", { name: "Set a value" }).click();
  await expect(count).toHaveValue("0");
  await expect(count).toHaveAttribute("aria-invalid", "false");
  await page.getByRole("button", { name: "Save 1 change" }).click();
  expect(
    JSON.parse(
      (await page.getByTestId("direct-submitted").textContent()) ?? "",
    ),
  ).toEqual({ type: "patch", input: { count: { op: "set", value: 0 } } });
});

test("nested null differs from an empty value in an expanded list editor", async ({
  page,
}) => {
  await page.goto("tests/components/direct-form.html");
  await page.getByRole("button", { name: "Edit nullable_values" }).click();
  const editor = page.getByRole("dialog", { name: "nullable values" });
  await expect(
    editor.getByText("Item 1").locator("..").getByText("Null", { exact: true }),
  ).toBeVisible();
  await expect(
    editor.getByLabel("nullable_values[0] value", { exact: true }),
  ).toHaveCount(0);
  await expect(
    editor.getByLabel("nullable_values[1] value", { exact: true }),
  ).toHaveValue("");
  await editor
    .getByRole("button", { name: "Set nullable_values[0] value" })
    .click();
  await editor
    .getByLabel("nullable_values[0] value", { exact: true })
    .fill("entered");
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Save 1 change" }).click();
  expect(
    JSON.parse(
      (await page.getByTestId("direct-submitted").textContent()) ?? "",
    ),
  ).toEqual({
    type: "patch",
    input: { nullable_values: { op: "set", value: ["entered", ""] } },
  });
});

test("large collections remain readable when editing is bounded", async ({
  page,
}) => {
  await page.goto("tests/components/direct-form.html");
  await page.getByRole("button", { name: "Edit large_values" }).click();
  const editor = page.getByRole("dialog", { name: "large values" });
  await expect(
    editor.getByText("Collection item limit reached. Editing is unavailable."),
  ).toBeVisible();
  await expect(editor.getByText(/item-0/)).toBeVisible();
  await expect(editor.getByText(/91 more items/)).toBeVisible();
});

test("descriptor change keeps existing collection available for read-only inspection", async ({
  page,
}) => {
  await page.goto("tests/components/direct-form.html");
  await page.getByRole("button", { name: "Change descriptor version" }).click();
  await expect(
    page.getByRole("button", { name: "Save changes" }),
  ).toBeDisabled();
  await expect(page.getByLabel("large_values options")).toBeDisabled();
  await page.getByRole("button", { name: "View large_values" }).click();
  const details = page.getByRole("dialog", { name: "large values" });
  await expect(details).toContainText("item-0");
  await expect(details.getByRole("textbox")).toHaveCount(0);
});

test("direct fields distinguish unchanged, false, zero, empty, null and removal", async ({
  page,
}) => {
  await page.goto("tests/components/direct-form.html");
  await expect(
    page.getByText(/No safe editor for unknown-opaque/),
  ).not.toBeVisible();
  await page.getByText("Advanced fields · 1 read-only").click();
  await page.getByLabel("opaque read-only details").click();
  await expect(
    page.getByText(/No safe editor for unknown-opaque/),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("button", { name: "Save changes" }),
  ).toBeDisabled();
  const enabled = page.getByRole("switch", { name: "enabled value" });
  await enabled.click();
  await enabled.click();
  await page.getByLabel("count value", { exact: true }).fill("0");
  await page.getByLabel("title value", { exact: true }).fill("");
  await page.getByLabel("note options").click();
  await page.getByRole("menuitem", { name: "Set null" }).click();
  await page.getByLabel("marker options").click();
  await page.getByRole("menuitem", { name: "Remove field" }).click();
  await expect(page.getByLabel("opaque options")).toHaveCount(0);
  await page.getByRole("button", { name: "Save 5 changes" }).click();
  expect(
    JSON.parse(
      (await page.getByTestId("direct-submitted").textContent()) ?? "",
    ),
  ).toEqual({
    type: "patch",
    input: {
      count: { op: "set", value: 0 },
      enabled: { op: "set", value: false },
      marker: { op: "remove" },
      note: { op: "set", value: null },
      title: { op: "set", value: "" },
    },
  });
});
