import { test, expect } from "@playwright/test";

test("standard fields provide shared compact controls without application renderers", async ({
  page,
}) => {
  await page.goto("tests/components/semantic-fields.html");
  await expect(
    page.getByRole("textbox", { name: "decimal value", exact: true }),
  ).toHaveValue("9007199254740993.000000000000000001");
  await expect(page.getByLabel("date value", { exact: true })).toHaveAttribute(
    "type",
    "date",
  );
  await expect(
    page.getByRole("textbox", { name: "time value", exact: true }),
  ).toHaveValue("12:00:00.123456789");
  await expect(
    page.getByLabel("Choose color", { exact: true }),
  ).toHaveAttribute("type", "color");
  await expect(
    page.getByRole("textbox", { name: "unit-value unit", exact: true }),
  ).toHaveValue("kg");
  await expect(
    page.getByText(/No renderer|Custom editor unavailable/),
  ).toHaveCount(0);
  await page
    .getByRole("textbox", { name: "decimal value", exact: true })
    .fill("9007199254740993.000000000000000002");
  await page
    .getByRole("textbox", { name: "datetime value", exact: true })
    .fill("2026-10-05T12:01:02.123456789+02:00");
  await page.getByRole("button", { name: /^Save (?:\d+ )?changes?$/ }).click();
  const patch = JSON.parse(
    (await page.getByLabel("Semantic patch").textContent()) ?? "null",
  );
  expect(patch.input).toMatchObject({
    decimal: { op: "set", value: "9007199254740993.000000000000000002" },
    datetime: { op: "set", value: "2026-10-05T10:01:02.123456789Z" },
  });
});

test("invalid semantic drafts block submission and retain the user's exact input", async ({
  page,
}) => {
  await page.goto("tests/components/semantic-fields.html");
  const input = page.getByRole("textbox", {
    name: "decimal value",
    exact: true,
  });
  await input.fill("1.");
  await expect(input).toHaveValue("1.");
  await expect(input).toHaveAttribute("aria-invalid", "true");
  await expect(
    page.getByRole("button", { name: /^Save (?:\d+ )?changes?$/ }),
  ).toBeDisabled();
  await page
    .getByRole("textbox", { name: "email value", exact: true })
    .fill("Alice@EXAMPLE.COM");
  await expect(input).toHaveValue("1.");
  await input.fill("0001.2300");
  await expect(
    page.getByRole("button", { name: /^Save (?:\d+ )?changes?$/ }),
  ).toBeEnabled();
  await expect(input).toHaveValue("0001.2300");
});

test("expanded document preserves numeric tokens and multiline source", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("tests/components/semantic-fields.html");
  await page
    .getByRole("button", { name: "Expand json-document editor", exact: true })
    .click();
  const json = page.getByRole("textbox", {
    name: "json-document expanded value",
    exact: true,
  });
  const text =
    '{\n  "number": 9007199254740993, "tiny": 0.000000000000000001\n}';
  await json.fill(text);
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: "Expand multiline editor", exact: true })
    .click();
  const multiline = page.getByRole("textbox", {
    name: "multiline expanded value",
    exact: true,
  });
  await expect(multiline).toHaveValue("one\ntwo");
  await multiline.fill("first\nsecond");
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: /^Save (?:\d+ )?changes?$/ }).click();
  const patch = JSON.parse(
    (await page.getByLabel("Semantic patch").textContent()) ?? "null",
  );
  expect(patch.input["json-document"].value).toBe(text);
  expect(patch.input.multiline.value).toBe("first\nsecond");
});

test("read-only semantic details expose the complete exact value", async ({
  page,
}) => {
  await page.goto("tests/components/semantic-fields.html");
  await page
    .getByRole("button", { name: "View full Archived document", exact: true })
    .click();
  const complete = page.getByRole("textbox", {
    name: "Archived document full value",
    exact: true,
  });
  await expect(complete).toHaveValue(
    '{"exact":9007199254740993,"notes":"' + "complete ".repeat(30) + 'END"}',
  );
  await expect(complete).toHaveAttribute("readonly", "");
  await page.keyboard.press("Escape");
  await expect(complete).toHaveCount(0);
});

test("compact measurement keeps its magnitude visible beside the unit", async ({
  page,
}) => {
  await page.goto("tests/components/semantic-fields.html");
  const input = page.getByRole("textbox", {
    name: "Compact measurement value",
    exact: true,
  });
  await expect(input).toHaveValue("12.5");
  const fits = await input.evaluate((element) => {
    const style = getComputedStyle(element);
    const canvas = document.createElement("canvas");
    const context = canvas.getContext("2d")!;
    context.font = style.font;
    return (
      element.clientWidth -
        parseFloat(style.paddingLeft) -
        parseFloat(style.paddingRight) >=
      context.measureText("12.5").width
    );
  });
  expect(fits).toBe(true);
  await expect(
    page.getByRole("textbox", {
      name: "Compact measurement unit",
      exact: true,
    }),
  ).toHaveValue("kg");
});
