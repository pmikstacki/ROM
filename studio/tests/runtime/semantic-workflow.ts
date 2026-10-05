import { expect, type Locator, type Page } from "@playwright/test";
import { selectValue } from "../components/select-value.ts";
import { parseWire, stringifyWire } from "../../src/lib/client/codec.ts";
import { projected } from "../../src/lib/client/validation.ts";
import type { WireObject } from "../../src/lib/client/types.ts";
import { humanLogin } from "./resource-workflow.ts";

const kind = "field-showcase";
const labels: Record<string, string> = {
  date: "Scheduled date",
  time: "Local time",
  recorded_at: "Recorded at",
  color: "color",
  email: "email",
  website: "Website",
  notes: "Notes",
  document: "JSON document",
  decimal: "decimal",
  measurement: "Measurement",
};
// Expected literals are independent of the browser normalization helper.
const entered = {
  date: "2028-02-29",
  time: "12:34:56.1200",
  recorded_at: "2028-02-29T12:34:56.12+02:30",
  color: "#AABBCCDD",
  email: "Case@EXAMPLE.COM",
  website: "https://EXAMPLE.COM",
  notes: "First line\nSecond line",
  document: '{"exact":900719925474099312345,"decimal":1.2300}',
  decimal: "-00012345678901234567890.1234500",
  measurement: { value: "00012.500", unit: "kg" },
};
const canonical = {
  date: "2028-02-29",
  time: "12:34:56.12",
  recorded_at: "2028-02-29T10:04:56.120000000Z",
  color: "#aabbccdd",
  email: "Case@example.com",
  website: "https://example.com/",
  notes: "First line\nSecond line",
  document: entered.document,
  decimal: "-12345678901234567890.12345",
  measurement: { value: "12.5", unit: "kg" },
};

async function api(page: Page, route: string, input: WireObject) {
  const response = await page.evaluate(
    async ({ route, body }) => {
      const session = await fetch("/rom-studio/auth/session", {
        credentials: "same-origin",
      });
      const auth = await session.json();
      const result = await fetch(`/rom-studio/api/${route}`, {
        method: "POST",
        credentials: "same-origin",
        headers: {
          "content-type": "application/json",
          "x-rom-csrf": auth.csrf_token,
        },
        body,
      });
      return { status: result.status, text: await result.text() };
    },
    { route, body: stringifyWire(input) },
  );
  return { status: response.status, value: parseWire(response.text) };
}
async function read(page: Page, id: string) {
  const result = await api(page, "read", { kind, id });
  expect(result.status).toBe(200);
  return projected(result.value, kind, id);
}
async function assertValues(
  page: Page,
  id: string,
  revision: bigint,
  overrides: WireObject = {},
) {
  const resource = await read(page, id);
  expect(resource.revision).toBe(revision);
  expect(resource.value).toMatchObject({ ...canonical, ...overrides });
}
async function fillValues(
  page: Page,
  form: Locator,
  action = false,
  modes = false,
) {
  for (const [field, value] of Object.entries(entered)) {
    const label = action ? field : labels[field];
    if (modes)
      await selectValue(
        form.getByLabel(`${label} mode`, { exact: true }),
        "value",
      );
    if (field === "notes" || field === "document") {
      await form
        .getByRole("button", { name: `Expand ${label} editor`, exact: true })
        .click();
      await page
        .getByLabel(`${label} expanded value`, { exact: true })
        .fill(value as string);
      await page.keyboard.press("Escape");
    } else if (typeof value === "string") {
      await form.getByLabel(`${label} value`, { exact: true }).fill(value);
    } else {
      await form
        .getByLabel(`${label} value`, { exact: true })
        .fill(value.value);
      await form.getByLabel(`${label} unit`, { exact: true }).fill(value.unit);
    }
  }
}
async function reopen(page: Page, id: string) {
  await page.getByRole("button", { name: `Open ${id}`, exact: true }).click();
  const discard = page.getByRole("button", {
    name: "Discard draft and reload",
    exact: true,
  });
  if (await discard.count()) await discard.click();
  return page.getByRole("region", { name: "Resource details", exact: true });
}

/** Actual production host: no intercepted API or fixture-only renderer data. */
export async function semanticWorkflow(page: Page, url: string) {
  await humanLogin(page, url);
  await page
    .getByRole("button", { name: "Field showcase", exact: true })
    .click();
  await expect(
    page.getByText("Workshop sample", { exact: true }).first(),
  ).toBeVisible();
  const id = "browser-semantic";
  await page
    .getByRole("button", { name: "Create Resource", exact: true })
    .click();
  const create = page.getByRole("region", {
    name: "Create Resource",
    exact: true,
  });
  await create.getByLabel("Resource ID", { exact: true }).fill(id);
  await selectValue(create.getByLabel("Name mode", { exact: true }), "value");
  await create
    .getByLabel("Name value", { exact: true })
    .fill("Browser semantic record");
  await fillValues(page, create, false, true);
  await selectValue(create.getByLabel("tags mode", { exact: true }), "value");
  await selectValue(
    create.getByLabel("Related task mode", { exact: true }),
    "value",
  );
  await create.getByLabel("Related task value", { exact: true }).fill("task-a");
  for (const label of ["Attachment", "category", "categories"])
    await selectValue(
      create.getByLabel(`${label} mode`, { exact: true }),
      "null",
    );
  await selectValue(
    create.getByLabel("related mode", { exact: true }),
    "value",
  );
  await create
    .getByRole("button", { name: "Choose related reference", exact: true })
    .click();
  await page.getByRole("option", { name: /Workshop sample/ }).click();
  await expect(create.getByLabel("related value", { exact: true })).toHaveValue(
    "workshop-sample",
  );
  await create
    .getByRole("button", { name: "Create resource", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: `Open ${id}`, exact: true }),
  ).toBeVisible();
  await assertValues(page, id, 1n);
  expect((await read(page, id)).value).toMatchObject({
    related: "workshop-sample",
  });

  // A changed draft exercises all ten patch controls, then returns canonical values.
  let details = await reopen(page, id);
  await details.getByLabel("decimal value", { exact: true }).fill("7");
  await details
    .getByRole("button", { name: "Save 1 change", exact: true })
    .click();
  await expect.poll(async () => (await read(page, id)).revision).toBe(2n);
  details = await reopen(page, id);
  await fillValues(page, details);
  await details
    .getByRole("button", { name: "Save 10 changes", exact: true })
    .click();
  await expect.poll(async () => (await read(page, id)).revision).toBe(3n);
  await assertValues(page, id, 3n);

  details = await reopen(page, id);
  const before = await api(page, "journal", { kind, after: null });
  expect(before.status).toBe(200);
  await details.getByLabel("decimal value", { exact: true }).fill("NaN");
  await expect(details.getByRole("button", { name: /^Save / })).toBeDisabled();
  await details
    .getByLabel("decimal value", { exact: true })
    .fill(canonical.decimal);
  const rejected = await api(page, "invoke", {
    kind,
    id,
    expected: 3n,
    idempotency: "semantic-invalid-decimal",
    operation: {
      type: "patch",
      input: { decimal: { op: "set", value: "NaN" } },
    },
  });
  expect(rejected.status).toBeGreaterThanOrEqual(400);
  await assertValues(page, id, 3n);
  const after = await api(page, "journal", { kind, after: null });
  expect(after.status).toBe(200);
  expect(after.value).toEqual(before.value);

  // Named typed action inputs use raw canonical field names, independently of labels.
  details = await reopen(page, id);
  const action = details.locator("form").filter({
    has: page.getByRole("heading", { name: "set_values", exact: true }),
  });
  await fillValues(page, action, true, true);
  await action.getByLabel("decimal value", { exact: true }).fill("0008.500");
  await action
    .getByRole("button", { name: "Run set_values", exact: true })
    .click();
  await expect.poll(async () => (await read(page, id)).revision).toBe(4n);
  await assertValues(page, id, 4n, { decimal: "8.5" });

  details = await reopen(page, id);
  const category = details.locator("form").filter({
    has: page.getByRole("heading", { name: "set_category", exact: true }),
  });
  await selectValue(
    category.getByLabel("input mode", { exact: true }),
    "value",
  );
  await category.getByLabel("input value", { exact: true }).click();
  await page
    .getByRole("option", { name: "Safety inspection", exact: true })
    .click();
  await category
    .getByRole("button", { name: "Run set_category", exact: true })
    .click();
  await expect
    .poll(async () => ((await read(page, id)).value as WireObject).category)
    .toBe("safety_check");

  details = await reopen(page, id);
  await details
    .getByRole("button", { name: "categories options", exact: true })
    .click();
  await page
    .getByRole("menuitem", { name: "Set a value", exact: true })
    .click();
  await details
    .getByRole("button", { name: "Edit categories", exact: true })
    .click();
  const list = page.getByRole("dialog", { name: "categories", exact: true });
  await list
    .getByRole("button", { name: "Choose categories items", exact: true })
    .click();
  await page
    .getByRole("checkbox", {
      name: "Select Preventive maintenance (preventive_maintenance) for categories",
      exact: true,
    })
    .check();
  await page
    .getByRole("checkbox", {
      name: "Select Reporting (reporting) for categories",
      exact: true,
    })
    .check();
  await page
    .getByRole("button", {
      name: "Append selected categories items",
      exact: true,
    })
    .click();
  await page.keyboard.press("Escape");
  await details
    .getByRole("button", { name: "Save 1 change", exact: true })
    .click();
  await expect
    .poll(async () => ((await read(page, id)).value as WireObject).categories)
    .toEqual(["preventive_maintenance", "reporting"]);

  await page
    .getByRole("button", { name: "Quick filters", exact: true })
    .click();
  await page
    .getByRole("dialog", { name: "Quick filters", exact: true })
    .getByRole("button", { name: "Open full filters", exact: true })
    .click();
  const filters = page.getByRole("region", {
    name: "Filter editor",
    exact: true,
  });
  await filters
    .getByRole("button", { name: "Add condition", exact: true })
    .click();
  await selectValue(
    filters.getByLabel("Filter 1 field", { exact: true }),
    "recorded_at",
  );
  await filters
    .getByLabel("Filter 1 value", { exact: true })
    .fill(entered.recorded_at);
  const query = page.waitForRequest((request) =>
    request.url().endsWith("/api/query"),
  );
  await filters
    .getByRole("button", { name: "Apply filters", exact: true })
    .click();
  const submitted = parseWire((await query).postData()!) as WireObject;
  expect(submitted.query).toMatchObject({
    filters: [{ field: "recorded_at", value: canonical.recorded_at }],
  });
  await expect(
    page.getByRole("button", { name: `Open ${id}`, exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Open workshop-sample", exact: true }),
  ).toHaveCount(0);
}
