import { test, expect } from "@playwright/test";
test("live revision retains draft and disables stale submission until explicit reload", async ({
  page,
}) => {
  await page.goto("tests/components/details.html");
  await page.getByLabel("title mode").selectOption("value");
  await page.getByLabel("title value", { exact: true }).fill("my draft");
  await page.getByRole("button", { name: "Advance live revision" }).click();
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "my draft",
  );
  await expect(
    page.getByRole("button", { name: "Apply patch" }),
  ).toBeDisabled();
  await expect(
    page.getByText("This Resource changed after the draft was opened."),
  ).toBeVisible();
  await page.getByRole("button", { name: "Discard draft and reload" }).click();
  await expect(page.getByRole("button", { name: "Apply patch" })).toBeEnabled();
  await page.getByLabel("title mode").selectOption("value");
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "other change",
  );
});
