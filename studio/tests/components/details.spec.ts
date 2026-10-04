import { selectValue } from "./select-value.ts";
import { test, expect } from "@playwright/test";
test("live revision retains draft and disables stale submission until explicit reload", async ({
  page,
}) => {
  await page.goto("tests/components/details.html");
  await selectValue(page.getByLabel("title mode"), "value");
  await page.getByLabel("title value", { exact: true }).fill("my draft");
  await page.getByRole("button", { name: "Advance live revision" }).click();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "my draft",
  );
  await expect(
    page.getByRole("button", { name: "Apply patch" }),
  ).toBeDisabled();
  await expect(
    page.getByText("This Resource changed after the draft was opened."),
  ).toBeVisible();
  await page.setViewportSize({ width: 1440, height: 900 });
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "my draft",
  );
  await expect(
    page.getByRole("button", { name: "Apply patch" }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Discard draft and reload" }).click();
  await expect(page.getByRole("button", { name: "Apply patch" })).toBeEnabled();
  await selectValue(page.getByLabel("title mode"), "value");
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "other change",
  );
});

test("an open field popup cannot mutate a draft after a newer revision", async ({
  page,
}) => {
  await page.goto("tests/components/details.html");
  await selectValue(page.getByLabel("title mode"), "value");
  await page.getByLabel("title value", { exact: true }).fill("preserved");
  await page.getByLabel("title mode").click();
  await page
    .getByRole("button", { name: "Advance live revision" })
    .evaluate((button) => {
      if (button instanceof HTMLElement) button.click();
    });
  const popupOption = page.getByRole("option", {
    name: "Unchanged / omitted",
    exact: true,
  });
  if (await popupOption.count()) {
    await expect(popupOption).toHaveAttribute("data-disabled", "");
    // Force a stale popup click to verify the adapter also rejects its callback.
    await popupOption.click({ force: true });
  }
  await expect(page.getByLabel("title mode")).toBeDisabled();
  await expect(page.getByLabel("title mode")).toContainText("Set value");
  await expect(
    page.getByRole("button", { name: "Apply patch" }),
  ).toBeDisabled();
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "preserved",
  );
});
