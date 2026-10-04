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

test("direct fields distinguish unchanged, false, zero, empty, null and removal", async ({
  page,
}) => {
  await page.goto("tests/components/direct-form.html");
  await expect(
    page.getByText(/No safe editor for unknown-opaque/),
  ).not.toBeVisible();
  await page.getByText("Advanced fields · 1 read-only").click();
  await expect(
    page.getByText(/No safe editor for unknown-opaque/),
  ).toBeVisible();
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
