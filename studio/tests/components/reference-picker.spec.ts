import { test, expect } from "@playwright/test";
const fixture = "tests/components/reference.html";
async function ready(page: import("@playwright/test").Page, suffix = "") {
  await page.goto(fixture + suffix);
  await expect(page.getByTestId("session")).toHaveText("ready");
}
test("reference lookup is lazy and chooses an authorized title without changing exact identity or main navigation", async ({
  page,
}) => {
  await ready(page);
  await expect(page.getByTestId("request-count")).toHaveText("0");
  await page
    .getByRole("button", { name: "Choose Assigned person reference" })
    .click();
  await expect(
    page.getByRole("option", { name: /Ada Lovelace/ }),
  ).toBeVisible();
  await page.getByRole("option", { name: /Ada Lovelace/ }).click();
  await expect(
    page.getByRole("searchbox", { name: "Assigned person value" }),
  ).toHaveValue("  id with spaces  ");
  await expect(page.getByTestId("stored-id")).toHaveText(
    '"  id with spaces  "',
  );
  await expect(page.getByTestId("selected-kind")).toHaveText("tasks");
  await expect(
    page.getByRole("button", { name: "Choose Assigned person reference" }),
  ).toBeFocused();
});
test("debounced search filters bounded candidates and Escape restores focus", async ({
  page,
}) => {
  await ready(page);
  await page
    .getByRole("button", { name: "Choose Assigned person reference" })
    .click();
  const search = page.getByRole("combobox", {
    name: "Search Assigned person candidates",
  });
  await expect(search).toBeFocused();
  await expect(
    page.getByRole("option", { name: /Ada Lovelace/ }),
  ).toBeVisible();
  await search.fill("Gr");
  await search.fill("Grace");
  await expect(
    page.getByRole("option", { name: /Grace Hopper/ }),
  ).toBeVisible();
  await expect(page.getByRole("option", { name: /Ada Lovelace/ })).toHaveCount(
    0,
  );
  await expect(page.getByTestId("request-count")).toHaveText("2");
  await search.press("ArrowDown");
  await search.press("Enter");
  await expect(page.getByTestId("stored-id")).toHaveText(
    '"α/people?exact#key"',
  );
  await page
    .getByRole("button", { name: "Choose Assigned person reference" })
    .click();
  await page
    .getByRole("combobox", { name: "Search Assigned person candidates" })
    .press("Escape");
  await expect(
    page.getByRole("button", { name: "Choose Assigned person reference" }),
  ).toBeFocused();
});
test("denied lookup preserves the exact ID fallback and never reports an empty successful search", async ({
  page,
}) => {
  await ready(page);
  await page.getByRole("button", { name: "Deny lookup", exact: true }).click();
  await page
    .getByRole("button", { name: "Choose Assigned person reference" })
    .click();
  await expect(
    page.getByText("Reference lookup was denied. Enter an exact Resource ID."),
  ).toBeVisible();
  await expect(
    page.getByText("No matches in these authorized candidates."),
  ).toHaveCount(0);
  await page.keyboard.press("Escape");
  await page
    .getByRole("searchbox", { name: "Assigned person value" })
    .fill(" manual/exact id ");
  await expect(page.getByTestId("stored-id")).toHaveText('" manual/exact id "');
});
test("withheld titles use the shared safe fallback and undisclosed kinds do not query", async ({
  page,
}) => {
  await ready(page, "?withheld");
  await expect(
    page.getByRole("button", { name: "Choose Private link reference" }),
  ).toHaveCount(0);
  await page
    .getByRole("searchbox", { name: "Private link value" })
    .fill(" untouched/id ");
  await expect(page.getByTestId("stored-hidden-id")).toHaveText(
    '" untouched/id "',
  );
  await page
    .getByRole("button", { name: "Choose Assigned person reference" })
    .click();
  await expect(
    page.getByRole("option", { name: /Person.*id with spaces/ }),
  ).toBeVisible();
  await expect(page.getByText("Ada Lovelace")).toHaveCount(0);
});
test("session invalidation removes delayed and selected descriptions without erasing an exact ID", async ({
  page,
}) => {
  await ready(page);
  await page.getByRole("button", { name: "Slow lookup", exact: true }).click();
  await page
    .getByRole("button", { name: "Choose Assigned person reference" })
    .click();
  await expect(page.getByText("Loading authorized candidates…")).toBeVisible();
  await page
    .getByRole("button", { name: "Disconnect session", exact: true })
    .click();
  await expect(page.getByTestId("session")).toHaveText("disconnected");
  // The injected transport ignores abort and returns after 600ms.
  await page.waitForTimeout(700);
  await expect(
    page.getByRole("button", { name: "Choose Assigned person reference" }),
  ).toHaveCount(0);
  await expect(page.getByText("Ada Lovelace")).toHaveCount(0);
  await expect(page.getByTestId("stored-id")).toHaveText('"existing-id"');
});
test("without lookup context the supported reference stays an exact ID editor", async ({
  page,
}) => {
  await ready(page, "?no-context");
  await expect(
    page.getByRole("button", { name: "Choose Assigned person reference" }),
  ).toHaveCount(0);
  await page
    .getByRole("searchbox", { name: "Assigned person value" })
    .fill("exact/ID?value");
  await expect(page.getByTestId("stored-id")).toHaveText('"exact/ID?value"');
  await expect(page.getByTestId("request-count")).toHaveText("0");
});
test("the picker remains inside a narrow mobile viewport", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 720 });
  await ready(page);
  await page
    .getByRole("button", { name: "Choose Assigned person reference" })
    .click();
  await expect(
    page.getByRole("option", { name: /Grace Hopper/ }),
  ).toBeVisible();
  const bounds = await page
    .locator('[data-slot="popover-content"]')
    .boundingBox();
  expect(bounds).not.toBeNull();
  expect(bounds!.x).toBeGreaterThanOrEqual(0);
  expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(320);
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth > window.innerWidth,
  );
  expect(overflow).toBe(false);
});

test("search requests are capped per open picker while exact ID editing remains available", async ({
  page,
}) => {
  await ready(page);
  await page
    .getByRole("button", { name: "Choose Assigned person reference" })
    .click();
  await expect(
    page.getByRole("option", { name: /Ada Lovelace/ }),
  ).toBeVisible();
  const search = page.getByRole("combobox", {
    name: "Search Assigned person candidates",
  });
  for (let index = 0; index < 7; index++) {
    await search.fill(`Absent ${index}`);
    await expect(page.getByTestId("request-count")).toHaveText(
      String(index + 2),
    );
  }
  await search.fill("Beyond request budget");
  await expect(
    page.getByText(
      "Reference search limit reached. Enter an exact Resource ID.",
    ),
  ).toBeVisible();
  await expect(page.getByTestId("request-count")).toHaveText("8");
  await search.press("Escape");
  await page
    .getByRole("searchbox", { name: "Assigned person value" })
    .fill("budget/exact-id");
  await expect(page.getByTestId("stored-id")).toHaveText('"budget/exact-id"');
});

test("selected authorized descriptions disappear with the session while the exact ID stays intact", async ({
  page,
}) => {
  await ready(page);
  await page
    .getByRole("button", { name: "Choose Assigned person reference" })
    .click();
  await page.getByRole("option", { name: /Ada Lovelace/ }).click();
  await expect(page.getByText("Ada Lovelace", { exact: true })).toBeVisible();
  await page
    .getByRole("button", { name: "Disconnect session", exact: true })
    .click();
  await expect(page.getByText("Ada Lovelace", { exact: true })).toHaveCount(0);
  await expect(page.getByTestId("stored-id")).toHaveText(
    '"  id with spaces  "',
  );
});

test("empty reference input reports the canonical field path without replacing the last value", async ({
  page,
}) => {
  await ready(page);
  await page.getByRole("searchbox", { name: "Assigned person value" }).fill("");
  await expect(page.getByTestId("field-error")).toContainText("assignee");
  await expect(page.getByTestId("stored-id")).toHaveText('"existing-id"');
});

test("a denied relookup retires the previously selected title and preserves the ID", async ({
  page,
}) => {
  await ready(page);
  await page
    .getByRole("button", { name: "Choose Assigned person reference" })
    .click();
  await page.getByRole("option", { name: /Ada Lovelace/ }).click();
  await page.getByRole("button", { name: "Deny lookup", exact: true }).click();
  await page
    .getByRole("button", { name: "Choose Assigned person reference" })
    .click();
  await expect(
    page.getByText("Reference lookup was denied. Enter an exact Resource ID."),
  ).toBeVisible();
  await expect(page.getByText("Ada Lovelace", { exact: true })).toHaveCount(0);
  await expect(page.getByTestId("stored-id")).toHaveText(
    '"  id with spaces  "',
  );
});
