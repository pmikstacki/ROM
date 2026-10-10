import { test, expect, type Page } from "@playwright/test";
async function creationTransport(page: Page, rows = 0) {
  const bodies: string[] = [];
  let lost = true;
  let version = 1;
  await page.route("**/creation-api/**", async (route) => {
    const endpoint = route.request().url().split("/").at(-1);
    if (endpoint === "discover")
      return route.fulfill({
        json: {
          version: 1,
          resources: [
            {
              kind: "notes",
              version,
              fields: [],
              actions: [],
              action_inputs: [],
            },
          ],
        },
      });
    if (endpoint === "query")
      return route.fulfill({
        json: Array.from({ length: rows }, (_, i) => ({
          key: { kind: "notes", id: `existing-${i}` },
          revision: 1,
          value: {},
        })),
      });
    bodies.push(route.request().postData()!);
    if (lost) return route.abort("failed");
    return route.fulfill({
      json: { key: { kind: "notes", id: "new-note" }, revision: 1, value: {} },
    });
  });
  return {
    bodies,
    confirm() {
      lost = false;
    },
    changeDefinition() {
      version = 2;
    },
  };
}
test("descriptor renewal keeps existing creation edits until explicit discard enables the current definition", async ({
  page,
}) => {
  const transport = await creationTransport(page);
  await page.goto("./tests/components/creation.html");
  await page
    .getByRole("button", { name: "Create Resource", exact: true })
    .click();
  const form = page.getByRole("region", {
    name: "Create Resource",
    exact: true,
  });
  const id = form.getByRole("textbox", { name: "Resource ID", exact: true });
  await id.fill("old-local-draft");
  await expect(
    form.getByText("Local draft saved", { exact: true }),
  ).toBeVisible();
  transport.changeDefinition();
  await page
    .getByRole("button", { name: "Renew creation session", exact: true })
    .click();
  await expect(form.getByText(/Resource definition changed/)).toBeVisible();
  await expect(id).toHaveValue("old-local-draft");
  await expect(
    form.getByRole("button", { name: "Create resource", exact: true }),
  ).toBeDisabled();
  expect(transport.bodies).toHaveLength(0);
  await form
    .getByRole("button", { name: "Discard local creation", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Discard local records", exact: true })
    .click();
  await expect(page.getByRole("alertdialog")).toBeHidden();
  await expect(id).toHaveValue("");
  await expect(id).toBeEnabled();
  await id.fill("new-note");
  await expect(
    form.getByText("Local draft saved", { exact: true }),
  ).toBeVisible();
  transport.confirm();
  await form
    .getByRole("button", { name: "Create resource", exact: true })
    .click();
  await expect(form).toBeHidden();
  expect(transport.bodies).toHaveLength(1);
});
test("descriptor renewal preserves exact pending creation for explicit retry", async ({
  page,
}) => {
  const transport = await creationTransport(page);
  await page.goto("./tests/components/creation.html");
  await page
    .getByRole("button", { name: "Create Resource", exact: true })
    .click();
  const form = page.getByRole("region", {
    name: "Create Resource",
    exact: true,
  });
  await form
    .getByRole("textbox", { name: "Resource ID", exact: true })
    .fill("new-note");
  await form
    .getByRole("button", { name: "Create resource", exact: true })
    .click();
  await expect(form.getByText(/Commit knowledge: unknown/)).toBeVisible();
  transport.changeDefinition();
  await page
    .getByRole("button", { name: "Renew creation session", exact: true })
    .click();
  await expect(form.getByText(/Resource definition changed/)).toBeVisible();
  expect(transport.bodies).toHaveLength(1);
  transport.confirm();
  await form
    .getByRole("button", { name: "Retry creation", exact: true })
    .click();
  await expect(form).toBeHidden();
  expect(transport.bodies).toHaveLength(2);
  expect(transport.bodies[1]).toBe(transport.bodies[0]);
});
test("pending creation disables row and page navigation without an uncaught rejection", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const transport = await creationTransport(page, 50);
  await page.goto("./tests/components/creation.html");
  const open = page.getByRole("button", {
    name: "Open existing-0",
    exact: true,
  });
  await expect(open).toBeEnabled();
  await expect(
    page.getByRole("button", { name: "Next page", exact: true }),
  ).toBeEnabled();
  await page
    .getByRole("button", { name: "Create Resource", exact: true })
    .click();
  const form = page.getByRole("region", {
    name: "Create Resource",
    exact: true,
  });
  await form
    .getByRole("textbox", { name: "Resource ID", exact: true })
    .fill("new-note");
  await form
    .getByRole("button", { name: "Create resource", exact: true })
    .click();
  await expect(form.getByText(/Commit knowledge: unknown/)).toBeVisible();
  await expect(open).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "Next page", exact: true }),
  ).toBeDisabled();
  await open.dispatchEvent("click");
  await page
    .getByRole("button", { name: "Next page", exact: true })
    .dispatchEvent("click");
  await expect(form.getByText(/Commit knowledge: unknown/)).toBeVisible();
  expect(errors).toEqual([]);
  expect(transport.bodies).toHaveLength(1);
  transport.confirm();
  await form
    .getByRole("button", { name: "Retry creation", exact: true })
    .click();
  await expect(form).toBeHidden();
  await expect(open).toBeEnabled();
  await expect(
    page.getByRole("button", { name: "Next page", exact: true }),
  ).toBeEnabled();
  expect(errors).toEqual([]);
});
test("confirmed creation after restart removes its original draft and closes the form", async ({
  page,
}) => {
  const transport = await creationTransport(page);
  await page.goto("./tests/components/creation.html");
  await page
    .getByRole("button", { name: "Create Resource", exact: true })
    .click();
  const form = page.getByRole("region", {
    name: "Create Resource",
    exact: true,
  });
  await form
    .getByRole("textbox", { name: "Resource ID", exact: true })
    .fill("new-note");
  await expect(
    form.getByText("Local draft saved", { exact: true }),
  ).toBeVisible();
  await form
    .getByRole("button", { name: "Create resource", exact: true })
    .click();
  await expect(form.getByText(/Commit knowledge: unknown/)).toBeVisible();
  expect(transport.bodies).toHaveLength(1);
  await page.reload();
  await page
    .getByRole("button", { name: "Create Resource", exact: true })
    .click();
  expect(transport.bodies).toHaveLength(1);
  await form
    .getByRole("button", { name: "Restore creation", exact: true })
    .click();
  await expect(
    form.getByRole("textbox", { name: "Resource ID", exact: true }),
  ).toHaveValue("new-note");
  transport.confirm();
  await form
    .getByRole("button", { name: "Retry creation", exact: true })
    .click();
  await expect.poll(() => transport.bodies.length).toBe(2);
  expect(transport.bodies[1]).toBe(transport.bodies[0]);
  await expect(form).toBeHidden();
  await page
    .getByRole("button", { name: "Create Resource", exact: true })
    .click();
  await form
    .getByRole("button", { name: "Restore creation", exact: true })
    .click();
  await expect(
    form.getByRole("textbox", { name: "Resource ID", exact: true }),
  ).toHaveValue("");
});
test("creation restores raw ID and accepted intent explicitly after reload without automatic dispatch", async ({
  page,
}) => {
  const transport = await creationTransport(page),
    bodies = transport.bodies;
  await page.goto("./tests/components/creation.html");
  await page
    .getByRole("button", { name: "Create Resource", exact: true })
    .click();
  const form = page.getByRole("region", {
    name: "Create Resource",
    exact: true,
  });
  await expect(
    form.getByRole("button", { name: "Restore creation", exact: true }),
  ).toBeVisible();
  await form
    .getByRole("textbox", { name: "Resource ID", exact: true })
    .fill("new-note");
  await form
    .getByRole("button", { name: "Create resource", exact: true })
    .click();
  await expect(form.getByText(/Commit knowledge: unknown/)).toBeVisible();
  expect(bodies).toHaveLength(1);
  await form
    .getByRole("textbox", { name: "Resource ID", exact: true })
    .fill("later-raw-draft");
  await expect(
    form.getByText("Local draft saved", { exact: true }),
  ).toBeVisible();
  await page.reload();
  await page
    .getByRole("button", { name: "Create Resource", exact: true })
    .click();
  expect(bodies).toHaveLength(1);
  await form
    .getByRole("button", { name: "Restore creation", exact: true })
    .click();
  await expect(
    form.getByRole("textbox", { name: "Resource ID", exact: true }),
  ).toHaveValue("later-raw-draft");
  await expect(form.getByText(/Commit knowledge: unknown/)).toBeVisible();
  expect(bodies).toHaveLength(1);
  transport.confirm();
  await form
    .getByRole("button", { name: "Retry creation", exact: true })
    .click();
  await expect.poll(() => bodies.length).toBe(2);
  expect(bodies[1]).toBe(bodies[0]);
  expect(bodies[0]).toContain('"expected":null');
  expect(bodies[0]).toContain("18446744073709551615");
  await expect(
    form.getByRole("textbox", { name: "Resource ID", exact: true }),
  ).toHaveValue("later-raw-draft");
  await form
    .getByRole("button", { name: "Discard local creation", exact: true })
    .click();
  await expect(page.getByRole("alertdialog")).toContainText(
    "It does not undo a backend commit.",
  );
  await page
    .getByRole("button", { name: "Keep creation", exact: true })
    .click();
  await expect(
    form.getByRole("textbox", { name: "Resource ID", exact: true }),
  ).toHaveValue("later-raw-draft");
});
