import { test, expect } from "@playwright/test";
const fixture = "tests/components/selection-layout-composition.html";
test("native cards support keyboard multi-selection and sibling actions do not toggle", async ({
  page,
}) => {
  await page.goto(fixture);
  const first = page.getByRole("button", {
      name: "Inspection equipment",
      exact: true,
    }),
    second = page.getByRole("button", {
      name: "Backup equipment",
      exact: true,
    });
  await first.focus();
  await page.keyboard.press("Space");
  await expect(first).toHaveAttribute("aria-pressed", "true");
  await second.focus();
  await page.keyboard.press("Enter");
  await expect(second).toHaveAttribute("aria-pressed", "true");
  await page
    .getByRole("button", { name: "Equipment details", exact: true })
    .click();
  await expect(page.getByLabel("Host calls")).toHaveText("2");
  await expect(page.getByLabel("Details calls")).toHaveText("1");
  await expect(page.locator("button button")).toHaveCount(0);
});
for (const viewport of [
  { width: 1280, height: 900 },
  { width: 390, height: 500 },
])
  test(`layout commands preserve host identity and bounds at ${viewport.width}`, async ({
    page,
  }) => {
    await page.setViewportSize(viewport);
    await page.goto(fixture);
    await expect(
      page.getByRole("button", {
        name: "Move left Inspection panel",
        exact: true,
      }),
    ).toBeDisabled();
    const right = page.getByRole("button", {
      name: "Move right Inspection panel",
      exact: true,
    });
    await right.focus();
    await page.keyboard.press("Space");
    await expect(page.getByLabel("Confirmed layout")).toContainText('"x":1');
    await page
      .getByRole("button", { name: "Widen Inspection panel", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Widen Inspection panel", exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: "Widen Inspection panel", exact: true }),
    ).toBeDisabled();
    await page
      .getByRole("button", { name: "Hide Inspection panel", exact: true })
      .click();
    await expect(page.getByLabel("Confirmed layout")).toContainText(
      '"visible":false',
    );
    const offenders = await page.evaluate(() =>
      [...document.querySelectorAll("main *")]
        .filter((el) => el.getBoundingClientRect().right > innerWidth + 1)
        .map((el) => ({
          tag: el.tagName,
          classes: el.className,
          width: el.getBoundingClientRect().width,
          text: el.textContent?.slice(0, 60),
        })),
    );
    expect(offenders).toEqual([]);
  });
for (const mode of ["Reject commands"])
  test(`${mode} is sanitized and never changes confirmed selection/layout`, async ({
    page,
  }) => {
    await page.goto(fixture);
    await page.getByRole("button", { name: mode, exact: true }).click();
    await page
      .getByRole("button", { name: "Inspection equipment", exact: true })
      .click();
    await expect(
      page.getByText("Change rejected", { exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Inspection equipment", exact: true }),
    ).toHaveAttribute("aria-pressed", "false");
    await page
      .getByRole("button", { name: "Move right Inspection panel", exact: true })
      .click();
    await expect(page.getByLabel("Confirmed layout")).toContainText('"x":0');
    await expect(
      page.getByText("private backend exception", { exact: true }),
    ).toHaveCount(0);
  });
test("unknown outcomes hold barriers until explicit host recovery and messages follow locale", async ({
  page,
}) => {
  await page.goto(fixture);
  await page
    .getByRole("button", { name: "Unknown commands", exact: true })
    .click();
  const card = page.getByRole("button", {
    name: "Inspection equipment",
    exact: true,
  });
  await card.click();
  await expect(card).toBeDisabled();
  await page
    .getByRole("button", { name: "Move right Inspection panel", exact: true })
    .click();
  await expect(
    page.getByRole("button", {
      name: "Move right Inspection panel",
      exact: true,
    }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Use Polish", exact: true }).click();
  await expect(page.getByText("Wynik nieznany", { exact: true })).toHaveCount(
    2,
  );
  await page
    .getByRole("button", { name: "Recover outcome", exact: true })
    .click();
  await expect(card).toBeEnabled();
  await card.click();
  await expect(card).toHaveAttribute("aria-pressed", "true");
});
test("scope change fences held callback failures and NaN token settles normally", async ({
  page,
}) => {
  await page.goto(fixture);
  await page
    .getByRole("button", { name: "Hold commands", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Inspection equipment", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Change authority", exact: true })
    .click();
  await page.getByRole("button", { name: "Reject held", exact: true }).click();
  await expect(page.getByText("Change rejected", { exact: true })).toHaveCount(
    0,
  );
  await page
    .getByRole("button", { name: "Recover outcome", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Use NaN authority", exact: true })
    .click();
  const card = page.getByRole("button", {
    name: "Inspection equipment",
    exact: true,
  });
  await card.click();
  await expect(card).toBeEnabled();
  await expect(card).toHaveAttribute("aria-pressed", "true");
});
test("disabled host controls cannot dispatch", async ({ page }) => {
  await page.goto(fixture);
  await page
    .getByRole("button", { name: "Disable controls", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Inspection equipment", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("button", {
      name: "Move right Inspection panel",
      exact: true,
    }),
  ).toBeDisabled();
  await expect(page.getByLabel("Host calls")).toHaveText("0");
});

test("disabling during a held callback does not strand the command after reenable", async ({
  page,
}) => {
  await page.goto(fixture);
  await page
    .getByRole("button", { name: "Hold commands", exact: true })
    .click();
  const card = page.getByRole("button", {
    name: "Inspection equipment",
    exact: true,
  });
  await card.click();
  await page
    .getByRole("button", { name: "Disable controls", exact: true })
    .click();
  await page.getByRole("button", { name: "Reject held", exact: true }).click();
  await page
    .getByRole("button", { name: "Enable controls", exact: true })
    .click();
  await expect(card).toBeEnabled();
});
test("layout callback is fenced across authority change and corrupt layout never falls back", async ({
  page,
}) => {
  await page.goto(fixture);
  await page
    .getByRole("button", { name: "Hold commands", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Move right Inspection panel", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Change authority", exact: true })
    .click();
  await page.getByRole("button", { name: "Reject held", exact: true }).click();
  await expect(page.getByText("Change rejected", { exact: true })).toHaveCount(
    0,
  );
  await page
    .getByRole("button", { name: "Corrupt layout", exact: true })
    .click();
  await expect(page.getByText("Invalid layout", { exact: true })).toBeVisible();
  await expect(
    page.getByRole("button", {
      name: "Move right Inspection panel",
      exact: true,
    }),
  ).toHaveCount(0);
});

test("lost acknowledgement after host work is unknown and cannot retry implicitly", async ({
  page,
}) => {
  await page.goto(fixture);
  await page
    .getByRole("button", { name: "Throw commands", exact: true })
    .click();
  const card = page.getByRole("button", {
    name: "Inspection equipment",
    exact: true,
  });
  await card.click();
  await expect(page.getByLabel("Accepted work")).toHaveText("1");
  await expect(
    page.getByText("Outcome unknown", { exact: true }),
  ).toBeVisible();
  await expect(card).toBeDisabled();
  await expect(
    page.getByText("private backend exception", { exact: true }),
  ).toHaveCount(0);
  await expect(card).toHaveAttribute("aria-pressed", "false");
  await page
    .getByRole("button", { name: "Move right Inspection panel", exact: true })
    .click();
  await expect(page.getByLabel("Accepted work")).toHaveText("2");
  await expect(page.getByText("Outcome unknown", { exact: true })).toHaveCount(
    2,
  );
  await expect(
    page.getByRole("button", {
      name: "Move right Inspection panel",
      exact: true,
    }),
  ).toBeDisabled();
  await expect(page.getByLabel("Confirmed layout")).toContainText('"x":0');
});
