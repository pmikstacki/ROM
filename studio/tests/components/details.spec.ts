import { test, expect } from "@playwright/test";
test("selected Resource shows its current title as a direct editable field", async ({
  page,
}) => {
  await page.goto("tests/components/details.html");
  await expect(
    page.getByRole("button", { name: "Save changes" }),
  ).toBeDisabled();
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "original",
  );
  await expect(page.getByLabel("title mode", { exact: true })).toHaveCount(0);
  await expect(
    page
      .getByRole("region", { name: "Resource details" })
      .getByText("title", { exact: true }),
  ).toHaveCount(1);
  await page.getByLabel("title value", { exact: true }).fill("new title");
  await expect(page.getByLabel("title options")).toHaveAttribute(
    "title",
    "Edited",
  );
  await expect(
    page.getByRole("button", { name: "Save 1 change" }),
  ).toBeEnabled();
});
test("direct boolean control keeps false and becomes an explicit edit only when switched", async ({
  page,
}) => {
  await page.goto("tests/components/details.html");
  const enabled = page.getByRole("switch", { name: "enabled value" });
  await expect(enabled).toHaveAttribute("aria-checked", "false");
  await expect(
    page.getByRole("button", { name: "Save changes" }),
  ).toBeDisabled();
  await enabled.click();
  await expect(enabled).toHaveAttribute("aria-checked", "true");
  await expect(
    page.getByRole("button", { name: "Save 1 change" }),
  ).toBeEnabled();
});
test("mobile Resource editor uses the full viewport and a concise heading", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("tests/components/details.html");
  await page.getByRole("button", { name: "Open details panel" }).click();
  const sheet = page.getByRole("dialog", { name: "Edit Resource" });
  await expect(sheet).toBeVisible();
  const box = await sheet.boundingBox();
  expect(box?.width).toBeGreaterThanOrEqual(385);
});
test("live revision retains draft and disables stale submission until explicit reload", async ({
  page,
}) => {
  await page.goto("tests/components/details.html");
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "original",
  );
  await page.getByLabel("title value", { exact: true }).fill("my draft");
  await page.getByRole("button", { name: "Advance live revision" }).click();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "my draft",
  );
  await expect(
    page.getByRole("button", { name: "Save 1 change" }),
  ).toBeDisabled();
  await expect(
    page.getByText("This Resource changed after the draft was opened."),
  ).toBeVisible();
  await page.setViewportSize({ width: 1440, height: 900 });
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "my draft",
  );
  await expect(
    page.getByRole("button", { name: "Save 1 change" }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Discard draft and reload" }).click();
  await expect(
    page.getByRole("button", { name: "Save changes" }),
  ).toBeDisabled();
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "other change",
  );
});

test("an open field popup cannot mutate a draft after a newer revision", async ({
  page,
}) => {
  await page.goto("tests/components/details.html");
  await page.getByLabel("title value", { exact: true }).fill("preserved");
  await page.getByLabel("title options").click();
  await page
    .getByRole("button", { name: "Advance live revision" })
    .evaluate((button) => {
      if (button instanceof HTMLElement) button.click();
    });
  const popupOption = page.getByRole("menuitem", {
    name: "Leave unchanged",
    exact: true,
  });
  if (await popupOption.count()) {
    await expect(popupOption).toHaveAttribute("data-disabled", "");
    // Force a stale popup click to verify the adapter also rejects its callback.
    await popupOption.click({ force: true });
  }
  await expect(page.getByLabel("title options")).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "Save 1 change" }),
  ).toBeDisabled();
  await expect(page.getByLabel("title value", { exact: true })).toHaveValue(
    "preserved",
  );
});

test("stale Resource keeps collection details readable without reopening mutation", async ({
  page,
}) => {
  await page.goto("tests/components/details.html");
  await page.getByRole("checkbox", { name: "Confirm deletion of one" }).click();
  await expect(
    page.getByRole("button", { name: "Delete Resource" }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Advance live revision" }).click();
  await expect(
    page.getByRole("button", { name: "Delete Resource" }),
  ).toBeDisabled();
  const view = page.getByRole("button", { name: "View labels" });
  await expect(view).toBeEnabled();
  await view.click();
  const details = page.getByRole("dialog", { name: "labels" });
  await expect(details).toContainText("first");
  await expect(details).toContainText("second");
  await expect(
    details.getByText("This value is read-only while editing is unavailable."),
  ).toBeVisible();
  await expect(details.getByRole("textbox")).toHaveCount(0);
  await expect(page.getByLabel("labels options")).toBeDisabled();
});
