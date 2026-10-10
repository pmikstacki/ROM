import { test, expect } from "@playwright/test";

test("installed public controls preserve bound values, native refs and keyboard operations", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Name", { exact: true }).fill("consumer");
  await page.getByLabel("Notes", { exact: true }).fill("consumer draft");
  await page.getByLabel("Choice", { exact: true }).selectOption("second");
  await page.getByRole("checkbox", { name: "Accept", exact: true }).focus();
  await page.keyboard.press("Space");
  await expect(page.getByRole("checkbox", { name: "Accept", exact: true })).toBeChecked();
  await page.getByRole("slider").focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.getByTestId("values")).toHaveText(JSON.stringify({ text: "consumer", draft: "consumer draft", selection: "second", checked: true, amount: 30 }));
  const refs = JSON.parse(await page.getByTestId("refs").innerText());
  expect(refs).toMatchObject({ input: "INPUT", textarea: "TEXTAREA", select: "SELECT", option: "OPTION", group: "OPTGROUP", button: "BUTTON", checkbox: "BUTTON", label: "LABEL" });
  expect(["SPAN", "DIV"]).toContain(refs.slider);
  await page.getByRole("button", { name: "Focus notes", exact: true }).click();
  await expect(page.getByLabel("Notes", { exact: true })).toBeFocused();
  await expect(page.getByLabel("Notes", { exact: true })).toHaveAttribute("aria-invalid", "true");
  await expect(page.getByLabel("Disabled input", { exact: true })).toBeDisabled();
  await expect(page.getByRole("button", { name: "Disabled button", exact: true })).toBeDisabled();
});

for (const viewport of [{ width: 1280, height: 900 }, { width: 390, height: 500 }]) {
  test(`installed stylesheet and wrapper contracts support a compact composer at ${viewport.width}x${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto("/");
    const name = page.getByLabel("Name", { exact: true });
    expect(await name.evaluate(el => getComputedStyle(el).borderRadius)).not.toBe("0px");
    expect(await name.evaluate(el => getComputedStyle(el).height)).toBe("36px");
    await expect(name).toHaveClass(/consumer-input/);
    const choice = page.getByLabel("Choice", { exact: true });
    await expect(choice.locator("..")).toHaveAttribute("data-slot", "native-select-wrapper");
    await expect(choice.locator("..")).toHaveClass(/consumer-select/);
    expect(await choice.evaluate(el => getComputedStyle(el).borderTopWidth)).toBe("3px");
    await page.getByLabel("Notes", { exact: true }).fill("Long draft\n".repeat(40));
    const geometry = await page.getByLabel("Notes", { exact: true }).evaluate(el => ({ height: el.getBoundingClientRect().height, width: el.getBoundingClientRect().width, overflow: getComputedStyle(el).overflowY, right: el.getBoundingClientRect().right }));
    expect(geometry.height).toBe(72);
    expect(geometry.right).toBeLessThanOrEqual(viewport.width);
    expect(geometry.overflow).toBe("auto");
  });
}

test("installed public client and recovery entries preserve exact typed mutation draft", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Prepare installed mutation", exact: true }).click();
  await expect(page.getByTestId("client-contract")).toHaveText(JSON.stringify({ phase: "prepared", unresolved: true, draft: '{"type":"replace","input":{"count":9007199254740993,"ratio":1.0,"open":false,"code":null}}', persisted: true }));
});
