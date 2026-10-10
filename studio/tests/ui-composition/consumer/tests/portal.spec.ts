import { test, expect } from "@playwright/test";

test("direct public guest entry never opens private storage and sums only explicitly selected guides", async ({
  page,
  request,
}) => {
  const published = await request.post("/api/invoke", {
    headers: { authorization: "Bearer fixture-alice" },
    data: {
      kind: "maintenance-guides",
      id: "electrical-second",
      expected: null,
      idempotency: "publish-second-guide",
      operation: {
        type: "create",
        input: {
          title: "Electrical inspection",
          nominal_voltage: 99999,
          updated_at: "2026-10-08T12:00:00Z",
          internal_notes: "PRIVATE PUBLISHER NOTE",
        },
      },
    },
  });
  expect(published.ok()).toBe(true);
  await page.addInitScript(() => {
    Object.defineProperty(window, "__fixtureStorageOpens", {
      value: 0,
      writable: true,
    });
    const original = indexedDB.open.bind(indexedDB);
    indexedDB.open = (...args) => {
      (window as unknown as { __fixtureStorageOpens: number })
        .__fixtureStorageOpens++;
      return original(...args);
    };
  });
  await page.goto("/?workspace=public");
  await expect(page.getByLabel("Current principal")).toHaveText("public-guest");
  await expect(page.getByLabel("Public guide observation")).toHaveText("fresh");
  const cards = page.getByRole("button", {
    name: "Electrical inspection",
    exact: true,
  });
  await expect(cards).toHaveCount(2);
  await expect(
    page.getByRole("button", {
      name: "Compute selected nominal values",
      exact: true,
    }),
  ).toBeDisabled();
  await cards.nth(0).click();
  await cards.nth(1).click();
  await page
    .getByRole("button", {
      name: "Compute selected nominal values",
      exact: true,
    })
    .click();
  await expect(page.getByLabel("Selected nominal value sum")).toHaveText(
    "100229",
  );
  await page
    .getByRole("button", {
      name: "Export selected public guide JSON",
      exact: true,
    })
    .click();
  await expect(page.getByLabel("Public export result")).toContainText(
    '"nominalValueSum":100229',
  );
  await cards.nth(1).click();
  await expect(page.getByLabel("Selected nominal value sum")).toHaveText(
    "none",
  );
  await expect(page.getByLabel("Public export result")).toHaveText("none");
  await expect(
    page.getByRole("button", {
      name: "Download public guide JSON",
      exact: true,
    }),
  ).toHaveCount(0);
  expect(
    await page.evaluate(
      () =>
        (window as unknown as { __fixtureStorageOpens: number })
          .__fixtureStorageOpens,
    ),
  ).toBe(0);
  const evidence = await (await request.get("/__fixture/evidence")).json();
  const browserRequests = evidence.requests.filter(
    (row) => row.route !== "invoke",
  );
  expect(
    browserRequests.every(
      (row) => row.subject === "public-guest" && !row.authorization_present,
    ),
  ).toBe(true);
  expect(
    evidence.requests.filter((row) => row.route === "invoke"),
  ).toHaveLength(1);
});

test("public guest anonymously observes, selects and computes exact guide snapshot JSON", async ({
  page,
  request,
}) => {
  await ready(page);
  await page
    .getByRole("button", { name: "Use public guest access", exact: true })
    .click();
  await expect(page.getByLabel("Public guide observation")).toHaveText("fresh");
  await expect(
    page.getByRole("region", { name: "Equipment", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Complete work order", exact: true }),
  ).toHaveCount(0);
  await page
    .getByRole("button", { name: "Electrical inspection", exact: true })
    .click();
  await page
    .getByRole("button", {
      name: "Compute selected nominal values",
      exact: true,
    })
    .click();
  await expect(page.getByLabel("Selected nominal value sum")).toHaveText("230");
  await page
    .getByRole("button", {
      name: "Export selected public guide JSON",
      exact: true,
    })
    .click();
  await expect(page.getByLabel("Public export result")).toContainText(
    '"revision":1',
  );
  await expect(page.getByLabel("Public export result")).toContainText(
    "2026-10-08T12:00:00.000000000Z",
  );
  await expect(page.getByLabel("Public export result")).not.toContainText(
    "internal_notes",
  );
  const download = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "Download public guide JSON", exact: true })
    .click();
  const file = await download;
  expect(file.suggestedFilename()).toBe("selected-public-guides.json");
  const stream = await file.createReadStream();
  const chunks = [];
  for await (const chunk of stream!) chunks.push(chunk);
  const saved = JSON.parse(Buffer.concat(chunks).toString());
  expect(saved.nominalValueSum).toBe(230);
  expect(saved.snapshots[0].value.value.nominal_voltage).toBe(230);
  expect(saved.snapshots[0].identity.resource).toEqual({
    kind: "maintenance-guides",
    id: "electrical",
    revision: 1,
  });
  expect(saved.snapshots[0].identity.selectedDate).toBe(
    "2026-10-08T12:00:00.000000000Z",
  );
  expect(JSON.stringify(saved)).not.toContain("internal_notes");
  const evidence = await (await request.get("/__fixture/evidence")).json();
  const publicRequests = evidence.requests.filter(
    (row) =>
      ["read", "query", "live"].includes(row.route) &&
      JSON.parse(row.body).kind === "maintenance-guides",
  );
  expect(publicRequests.some((row) => row.route === "read")).toBe(true);
  expect(publicRequests.some((row) => row.route === "query")).toBe(true);
  expect(publicRequests.some((row) => row.route === "live")).toBe(true);
  expect(
    publicRequests.every(
      (row) =>
        row.subject === "public-guest" && row.authorization_present === false,
    ),
  ).toBe(true);
  const invokeCount = evidence.requests.filter(
    (row) => row.route === "invoke",
  ).length;
  expect(invokeCount).toBe(0);
  const denied = await request.post("/api/invoke", {
    headers: { "X-Subject": "alice" },
    data: {
      kind: "maintenance-guides",
      id: "electrical",
      expected: 1,
      idempotency: "guest-blocked",
      operation: {
        type: "replace",
        input: {
          title: "Guest changed",
          nominal_voltage: 1,
          updated_at: "2026-10-08T12:00:00Z",
          internal_notes: "",
        },
      },
    },
  });
  expect(denied.status()).toBe(403);
  const privateRead = await request.post("/api/read", {
    headers: { "X-Subject": "alice" },
    data: {
      kind: "equipment",
      id: "pump-1",
    },
  });
  expect(privateRead.status()).toBe(403);
  const spoofedRead = await request.post("/api/read", {
    data: {
      kind: "equipment",
      id: "pump-1",
      actor: { authority: "maintenance-portal", subject: "alice" },
    },
  });
  expect(spoofedRead.status()).toBe(400);
  expect(await spoofedRead.text()).not.toContain("Feed pump");
  const hiddenQuery = await request.post("/api/query", {
    data: {
      kind: "maintenance-guides",
      field: "internal_notes",
      value: "PRIVATE PUBLISHER NOTE",
    },
  });
  expect(hiddenQuery.status()).toBe(403);
  const unverified = await request.post("/api/read", {
    headers: { authorization: "Bearer unverified" },
    data: { kind: "maintenance-guides", id: "electrical" },
  });
  expect(unverified.status()).toBe(403);
  const journal = await request.post("/api/journal", {
    headers: { authorization: "Bearer fixture-alice" },
    data: { kind: "maintenance-guides", after: null },
  });
  expect((await journal.json()).events).toHaveLength(1);
});

test("held public guide export cannot publish after switching to a private context", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await ready(page);
  await page
    .getByRole("button", { name: "Use public guest access", exact: true })
    .click();
  await expect(page.getByLabel("Public guide observation")).toHaveText("fresh");
  await page
    .getByRole("button", { name: "Electrical inspection", exact: true })
    .click();
  await request.post("/__fixture/control", {
    data: { type: "hold", route: "read", kind: "maintenance-guides" },
  });
  await page
    .getByRole("button", {
      name: "Export selected public guide JSON",
      exact: true,
    })
    .click();
  await expect
    .poll(
      async () =>
        (await (await request.get("/__fixture/evidence")).json()).held,
    )
    .toBe(1);
  await page
    .getByRole("button", { name: "Use Alice fixture identity", exact: true })
    .click();
  await request.post("/__fixture/control", { data: { type: "release" } });
  await expect(page.getByLabel("Equipment observation")).toHaveText("fresh");
  await expect(page.getByLabel("Public export result")).toHaveCount(0);
  await expect(
    page.getByRole("button", {
      name: "Download public guide JSON",
      exact: true,
    }),
  ).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("WorkOrder lost acknowledgement restores the original intent after reload and authority change", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await ready(page);
  await request.post("/__fixture/control", {
    data: { type: "drop-ack", route: "invoke", kind: "work-orders" },
  });
  await page
    .getByRole("button", { name: "Complete work order", exact: true })
    .click();
  await expect(page.getByLabel("Work save")).toHaveText("unknown");
  await request.post("/__fixture/control", { data: { type: "restart" } });
  await page.reload();
  await expect(page.getByLabel("Equipment observation")).toHaveText("fresh");
  const restore = page.getByRole("button", {
    name: "Restore pending work order",
    exact: true,
  });
  await expect(restore).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Complete work order", exact: true }),
  ).toBeDisabled();
  await page.evaluate(() => {
    const prototype = IDBDatabase.prototype;
    Object.defineProperty(prototype, "__fixtureOriginalTransaction", {
      value: prototype.transaction,
      configurable: true,
    });
    prototype.transaction = () => {
      throw Error("private Work storage failure");
    };
  });
  await restore.click();
  await expect(
    page.getByText("Pending work order restore unavailable", { exact: true }),
  ).toBeVisible();
  await expect(restore).toBeVisible();
  await expect(
    page.getByText("Pending Settings restore unavailable", { exact: true }),
  ).toHaveCount(0);
  expect(errors).toEqual([]);
  await page.evaluate(() => {
    const prototype = IDBDatabase.prototype as IDBDatabase & {
      __fixtureOriginalTransaction: IDBDatabase["transaction"];
    };
    prototype.transaction = prototype.__fixtureOriginalTransaction;
    delete (prototype as unknown as Record<string, unknown>)
      .__fixtureOriginalTransaction;
  });
  await restore.click();
  await expect(page.getByLabel("Work save")).toHaveText("unknown");
  await page
    .getByRole("button", { name: "Use Bob fixture identity", exact: true })
    .click();
  await expect(page.getByLabel("Current principal")).toHaveText("bob");
  await expect(
    page.getByRole("button", {
      name: "Retry original work order",
      exact: true,
    }),
  ).toHaveCount(0);
  await expect(restore).toHaveCount(0);
  const before = await (await request.get("/__fixture/evidence")).json();
  expect(
    before.requests.filter(
      (row) =>
        row.route === "invoke" && JSON.parse(row.body).kind === "work-orders",
    ),
  ).toHaveLength(1);
  const original = before.requests.find(
    (row) =>
      row.route === "invoke" && JSON.parse(row.body).kind === "work-orders",
  );
  const denied = await request.post("/api/invoke", {
    headers: { authorization: "Bearer fixture-bob" },
    data: JSON.parse(original.body),
  });
  expect(denied.status()).toBe(403);
  expect(await denied.text()).not.toContain("Replace seal");
  await page
    .getByRole("button", { name: "Use Alice fixture identity", exact: true })
    .click();
  await expect(restore).toBeVisible();
  await restore.click();
  await expect(page.getByLabel("Work save")).toHaveText("unknown");
  const reply = page.waitForResponse((response) =>
    response.url().endsWith("/api/invoke"),
  );
  await page
    .getByRole("button", { name: "Retry original work order", exact: true })
    .click();
  const receipt = await (await reply).json();
  expect(receipt.revision).toBe(2);
  expect(receipt.value.completed).toBe(true);
  await expect(page.getByLabel("Work save")).toHaveText("succeeded");
  const evidence = await (await request.get("/__fixture/evidence")).json();
  const invocations = evidence.requests.filter(
    (row) =>
      row.route === "invoke" &&
      row.subject === "alice" &&
      JSON.parse(row.body).kind === "work-orders",
  );
  expect(invocations).toHaveLength(2);
  expect(invocations[1].body).toBe(invocations[0].body);
  const journal = await request.post("/api/journal", {
    headers: { authorization: "Bearer fixture-alice" },
    data: { kind: "work-orders", after: null },
  });
  expect((await journal.json()).events).toHaveLength(2);
  expect(errors).toEqual([]);
});

test("held original WorkOrder replay is fenced when its principal context changes", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await ready(page);
  await request.post("/__fixture/control", {
    data: { type: "drop-ack", route: "invoke", kind: "work-orders" },
  });
  await page
    .getByRole("button", { name: "Complete work order", exact: true })
    .click();
  await expect(page.getByLabel("Work save")).toHaveText("unknown");
  await request.post("/__fixture/control", {
    data: { type: "hold", route: "invoke", kind: "work-orders" },
  });
  await page
    .getByRole("button", { name: "Retry original work order", exact: true })
    .click();
  await expect
    .poll(
      async () =>
        (await (await request.get("/__fixture/evidence")).json()).held,
    )
    .toBe(1);
  await page
    .getByRole("button", { name: "Use Bob fixture identity", exact: true })
    .click();
  await request.post("/__fixture/control", { data: { type: "release" } });
  await expect(page.getByLabel("Current principal")).toHaveText("bob");
  await expect(page.getByLabel("Work save")).toHaveText("idle");
  await expect(
    page.getByRole("button", {
      name: "Retry original work order",
      exact: true,
    }),
  ).toHaveCount(0);
  await expect(page.getByText(/Replace seal/)).toHaveCount(0);
  const evidence = await (await request.get("/__fixture/evidence")).json();
  const invocations = evidence.requests.filter(
    (row) =>
      row.route === "invoke" && JSON.parse(row.body).kind === "work-orders",
  );
  expect(invocations).toHaveLength(2);
  expect(invocations.every((row) => row.subject === "alice")).toBe(true);
  expect(invocations[1].body).toBe(invocations[0].body);
  expect(errors).toEqual([]);
});

test("confirmed layout moves and resizes the displayed history grid panel", async ({
  page,
}) => {
  await ready(page);
  await page.getByLabel("Workspace role").selectOption("dispatcher");
  const panel = page.getByRole("region", {
    name: "History panel",
    exact: true,
  });
  const bounds = () =>
    panel.evaluate((element) => {
      const rect = element.getBoundingClientRect();
      return {
        x: rect.left + scrollX,
        y: rect.top + scrollY,
        width: rect.width,
        height: rect.height,
      };
    });
  const before = await bounds();
  expect(before).not.toBeNull();
  await page
    .getByRole("button", { name: "Move right Inspection history", exact: true })
    .click();
  await expect(page.getByLabel("Confirmed history position")).toHaveText("1");
  await expect
    .poll(async () => (await bounds()).x)
    .toBeGreaterThan(before.x + 20);
  const moved = await bounds();
  await page
    .getByRole("button", { name: "Make wider Inspection history", exact: true })
    .click();
  await expect
    .poll(async () => (await bounds()).width)
    .toBeGreaterThan(moved.width + 20);
  const wider = await bounds();
  await page
    .getByRole("button", {
      name: "Make taller Inspection history",
      exact: true,
    })
    .click();
  await expect
    .poll(async () => (await bounds()).height)
    .toBeGreaterThan(wider.height + 20);
  const taller = await bounds();
  await page
    .getByRole("button", { name: "Move down Inspection history", exact: true })
    .click();
  await expect
    .poll(async () => (await bounds()).y)
    .toBeGreaterThan(taller.y + 20);
  const positioned = await bounds();
  await page.reload();
  await expect(page.getByLabel("Equipment observation")).toHaveText("fresh");
  await expect
    .poll(async () => (await bounds()).x)
    .toBeCloseTo(positioned.x, 0);
  await expect
    .poll(async () => (await bounds()).y)
    .toBeCloseTo(positioned.y, 0);
  await expect
    .poll(async () => (await bounds()).width)
    .toBeCloseTo(positioned.width, 0);
  await expect
    .poll(async () => (await bounds()).height)
    .toBeCloseTo(positioned.height, 0);
});

test.beforeEach(async ({ request }) => {
  const reset = await request.post("/__fixture/control", {
    data: { type: "reset" },
  });
  expect(reset.ok()).toBe(true);
});

async function ready(page) {
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Maintenance workspace" }),
  ).toBeVisible();
  await expect(page.getByLabel("Equipment observation")).toHaveText("fresh");
  await expect(page.getByLabel("Current principal")).toHaveText("alice");
}

test("technician selects equipment, edits exact UTC date/reference and exports committed inspection", async ({
  page,
}) => {
  await ready(page);
  const card = page.getByRole("button", { name: "Feed pump", exact: true });
  await card.focus();
  await page.keyboard.press("Space");
  await expect(card).toHaveAttribute("aria-pressed", "true");
  await page
    .getByRole("button", { name: "Inspect Feed pump", exact: true })
    .click();
  await expect(card).toHaveAttribute("aria-pressed", "true");
  await page.getByLabel("Inspection notes").fill("Seal reviewed by technician");
  await page.getByLabel("Inspection UTC instant").fill("2026-10-09T14:30:00Z");
  await page.getByLabel("Related equipment value").fill("pump-1");
  await page
    .getByRole("button", { name: "Save inspection", exact: true })
    .click();
  await expect(page.getByLabel("Inspection save")).toHaveText("succeeded");
  await expect(page.getByLabel("Confirmed inspection date")).toHaveText(
    "2026-10-09T14:30:00.000000000Z",
  );
  await page
    .getByRole("button", { name: "Export inspection JSON", exact: true })
    .click();
  await expect(page.getByLabel("Export result")).toContainText(
    "Seal reviewed by technician",
  );
  await expect(page.getByLabel("Export result")).toContainText(
    "2026-10-09T14:30:00.000000000Z",
  );
  await expect(page.getByLabel("Export result")).toContainText(
    '"id":"check-1"',
  );
  await expect(page.getByLabel("Export result")).toContainText('"revision":2');
  const download = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "Download captured JSON", exact: true })
    .click();
  expect((await download).suggestedFilename()).toBe("inspection-check-1.json");
});

test("reference lookup uses authorized exact IDs and history reads real committed revisions", async ({
  page,
}) => {
  await ready(page);
  await page
    .getByRole("button", { name: "Inspect Feed pump", exact: true })
    .click();
  await page
    .getByRole("button", {
      name: "Choose Related equipment reference",
      exact: true,
    })
    .click();
  await page.getByRole("option", { name: /Feed pump/ }).click();
  await expect(page.getByLabel("Related equipment value")).toHaveValue(
    "pump-1",
  );
  await page
    .getByRole("button", { name: "Close inspection details", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Refresh inspection history", exact: true })
    .click();
  await expect(
    page.getByRole("list", { name: "Inspection history" }),
  ).toBeVisible();
  await page
    .getByRole("button", {
      name: "Inspection check-1 · revision 1",
      exact: true,
    })
    .click();
  await expect(page.getByLabel("Selected history revision")).toHaveText("1");
});

test("dispatcher keyboard layout stores confirmed Settings and survives reload", async ({
  page,
}) => {
  await ready(page);
  await page.getByLabel("Workspace role").selectOption("dispatcher");
  const right = page.getByRole("button", {
    name: "Move right Inspection history",
    exact: true,
  });
  await right.focus();
  await page.keyboard.press("Space");
  await expect(page.getByLabel("Settings save")).toHaveText("succeeded");
  await expect(page.getByLabel("Confirmed history position")).toHaveText("1");
  await page.reload();
  await expect(page.getByLabel("Equipment observation")).toHaveText("fresh");
  await page.getByLabel("Workspace role").selectOption("dispatcher");
  await expect(page.getByLabel("Confirmed history position")).toHaveText("1");
  await page
    .getByRole("button", { name: "Hide Inspection history", exact: true })
    .click();
  await expect(page.getByLabel("Settings save")).toHaveText("succeeded");
  await page.reload();
  await expect(page.getByLabel("Equipment observation")).toHaveText("fresh");
  await expect(page.getByLabel("History visibility")).toHaveText("hidden");
  await expect(
    page.getByRole("region", { name: "History panel", exact: true }),
  ).toBeHidden();
});

test("mobile details preserve draft across resize and return focus on Escape", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await ready(page);
  const opener = page.getByRole("button", {
    name: "Inspect Feed pump",
    exact: true,
  });
  await opener.click();
  await page
    .getByLabel("Inspection notes")
    .fill("Unsubmitted technician draft");
  await page.setViewportSize({ width: 390, height: 500 });
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(page.getByLabel("Inspection notes")).toHaveValue(
    "Unsubmitted technician draft",
  );
  await page.keyboard.press("Escape");
  await expect(opener).toBeFocused();
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth > innerWidth + 1,
  );
  expect(overflow).toBe(false);
});

test("principal change clears private selected IDs, drafts, reference labels, history and export", async ({
  page,
}) => {
  await ready(page);
  await page.getByRole("button", { name: "Feed pump", exact: true }).click();
  await page
    .getByRole("button", { name: "Inspect Feed pump", exact: true })
    .click();
  await page.getByLabel("Inspection notes").fill("Private Alice draft");
  await page
    .getByRole("button", { name: "Close inspection details", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Refresh inspection history", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Export inspection JSON", exact: true })
    .click();
  await expect(page.getByLabel("Export result")).toContainText("check-1");
  await page
    .getByRole("button", { name: "Use Bob fixture identity", exact: true })
    .click();
  await expect(page.getByLabel("Current principal")).toHaveText("bob");
  await expect(page.getByLabel("Selected equipment")).toHaveText("none");
  await expect(page.getByLabel("Export result")).toHaveText("none");
  await expect(
    page.getByText("Private Alice draft", { exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("list", { name: "Inspection history" }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Feed pump", exact: true }),
  ).toHaveCount(0);
});

test("lost acknowledgement survives server/browser restart and retries only the retained exact invocation", async ({
  page,
  request,
}) => {
  await ready(page);
  await page
    .getByRole("button", { name: "Inspect Feed pump", exact: true })
    .click();
  await page
    .getByLabel("Inspection notes")
    .fill("Committed with lost acknowledgement");
  await request.post("/__fixture/control", {
    data: { type: "drop-ack", route: "invoke", kind: "inspections" },
  });
  await page
    .getByRole("button", { name: "Save inspection", exact: true })
    .click();
  await expect(page.getByLabel("Inspection save")).toHaveText("unknown");
  await expect(
    page.getByRole("button", { name: "Save inspection", exact: true }),
  ).toBeDisabled();
  await request.post("/__fixture/control", { data: { type: "restart" } });
  await page.reload();
  await expect(page.getByLabel("Equipment observation")).toHaveText("fresh");
  await page
    .getByRole("button", { name: "Inspect Feed pump", exact: true })
    .click();
  await expect(
    page.getByRole("button", {
      name: "Restore pending inspection",
      exact: true,
    }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Restore pending inspection", exact: true })
    .click();
  await expect(page.getByLabel("Inspection save")).toHaveText("unknown");
  await page
    .getByRole("button", { name: "Retry original inspection", exact: true })
    .click();
  await expect(page.getByLabel("Inspection save")).toHaveText("succeeded");
  const evidence = await (await request.get("/__fixture/evidence")).json();
  const invocations = evidence.requests.filter(
    (row) =>
      row.route === "invoke" && JSON.parse(row.body).kind === "inspections",
  );
  expect(invocations).toHaveLength(2);
  expect(invocations[1].body).toBe(invocations[0].body);
  const journal = await request.post("/api/journal", {
    headers: { authorization: "Bearer fixture-alice" },
    data: { kind: "inspections", after: null },
  });
  expect((await journal.json()).events).toHaveLength(2);
});

test("held authorized export cannot disclose into another principal", async ({
  page,
  request,
}) => {
  await ready(page);
  await request.post("/__fixture/control", {
    data: { type: "hold", route: "read", kind: "inspections" },
  });
  await page
    .getByRole("button", { name: "Export inspection JSON", exact: true })
    .click();
  await expect
    .poll(
      async () =>
        (await (await request.get("/__fixture/evidence")).json()).held,
    )
    .toBe(1);
  await page
    .getByRole("button", { name: "Use Bob fixture identity", exact: true })
    .click();
  await request.post("/__fixture/control", { data: { type: "release" } });
  await expect(page.getByLabel("Current principal")).toHaveText("bob");
  await expect(page.getByLabel("Export result")).toHaveText("none");
  await expect(
    page.getByRole("button", { name: "Feed pump", exact: true }),
  ).toHaveCount(0);
});

test("invalid date adds no event and unverified fixture identity cannot write", async ({
  page,
  request,
}) => {
  await ready(page);
  await page
    .getByRole("button", { name: "Inspect Feed pump", exact: true })
    .click();
  await page.getByLabel("Inspection UTC instant").fill("yesterday");
  await page
    .getByRole("button", { name: "Save inspection", exact: true })
    .click();
  await expect(page.getByLabel("Inspection save")).toHaveText("rejected");
  const journal = await request.post("/api/journal", {
    headers: { authorization: "Bearer fixture-alice" },
    data: { kind: "inspections", after: null },
  });
  expect((await journal.json()).events).toHaveLength(1);
  const denied = await request.post("/api/invoke", {
    headers: { authorization: "Bearer unverified" },
    data: {
      kind: "portal-settings",
      id: "workspace",
      expected: 1,
      idempotency: "guest-write",
      operation: {
        type: "replace",
        input: {
          owner: "alice",
          columns: 4,
          show_history: false,
          layout: "[]",
        },
      },
    },
  });
  expect(denied.status()).toBe(403);
});

test("visibility adapter pauses the real stream and resumes with a fresh authorized snapshot", async ({
  page,
  request,
}) => {
  await ready(page);
  await page.evaluate(() => {
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "hidden",
    });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await expect(page.getByLabel("Equipment observation")).toHaveText("stale");
  await expect(
    page.getByRole("button", { name: "Feed pump", exact: true }),
  ).toBeVisible();
  const changed = await request.post("/api/invoke", {
    headers: { authorization: "Bearer fixture-alice" },
    data: {
      kind: "equipment",
      id: "pump-1",
      expected: 1,
      idempotency: "visibility-update",
      operation: {
        type: "replace",
        input: { owner: "alice", title: "Feed pump after pause", active: true },
      },
    },
  });
  expect(changed.ok()).toBe(true);
  await expect(
    page.getByRole("button", { name: "Feed pump", exact: true }),
  ).toBeVisible();
  await page.evaluate(() => {
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "visible",
    });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await expect(page.getByLabel("Equipment observation")).toHaveText("fresh");
  await expect(
    page.getByRole("button", { name: "Feed pump after pause", exact: true }),
  ).toBeVisible();
});

test("public recovery executes WorkOrder completion and corrupt layout is rejected by server policy", async ({
  page,
  request,
}) => {
  await ready(page);
  await page
    .getByRole("button", { name: "Complete work order", exact: true })
    .click();
  await expect(page.getByLabel("Work save")).toHaveText("succeeded");
  await expect(
    page.getByRole("button", { name: "Complete work order", exact: true }),
  ).toBeDisabled();
  const bad = await request.post("/api/invoke", {
    headers: { authorization: "Bearer fixture-alice" },
    data: {
      kind: "portal-settings",
      id: "workspace",
      expected: 1,
      idempotency: "invalid-catalog",
      operation: {
        type: "replace",
        input: {
          owner: "alice",
          columns: 4,
          show_history: true,
          layout: JSON.stringify([
            { id: "unknown", x: 0, y: 0, width: 2, height: 1, visible: true },
          ]),
        },
      },
    },
  });
  expect(bad.status()).toBe(400);
  await page.getByLabel("Workspace role").selectOption("dispatcher");
  await expect(page.getByLabel("Confirmed history position")).toHaveText("0");
});

test("temporary history refusal is sanitized without an unhandled callback rejection", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await ready(page);
  await request.post("/__fixture/control", {
    data: { type: "unavailable", route: "journal", kind: "inspections" },
  });
  await page
    .getByRole("button", { name: "Refresh inspection history", exact: true })
    .click();
  await expect(
    page.getByText("Inspection history unavailable", { exact: true }),
  ).toBeVisible();
  expect(errors).toEqual([]);
  await expect(
    page.getByRole("button", { name: "Feed pump", exact: true }),
  ).toBeVisible();
});

test("failed explicit restore keeps durable original intent and suppresses private storage failure", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await ready(page);
  await page
    .getByRole("button", { name: "Inspect Feed pump", exact: true })
    .click();
  await page
    .getByLabel("Inspection notes")
    .fill("Original unresolved inspection");
  await request.post("/__fixture/control", {
    data: { type: "drop-ack", route: "invoke", kind: "inspections" },
  });
  await page
    .getByRole("button", { name: "Save inspection", exact: true })
    .click();
  await expect(page.getByLabel("Inspection save")).toHaveText("unknown");
  await page.reload();
  await expect(page.getByLabel("Equipment observation")).toHaveText("fresh");
  await page
    .getByRole("button", { name: "Inspect Feed pump", exact: true })
    .click();
  await expect(
    page.getByRole("button", {
      name: "Restore pending inspection",
      exact: true,
    }),
  ).toBeVisible();
  await page.evaluate(() => {
    const prototype = IDBDatabase.prototype;
    Object.defineProperty(prototype, "__fixtureOriginalTransaction", {
      value: prototype.transaction,
      configurable: true,
    });
    prototype.transaction = () => {
      throw Error("private fixture storage failure");
    };
  });
  await page
    .getByRole("button", { name: "Restore pending inspection", exact: true })
    .click();
  await expect(
    page.getByText("Pending inspection restore unavailable", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Save inspection", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByText("private fixture storage failure", { exact: true }),
  ).toHaveCount(0);
  expect(errors).toEqual([]);
  await page.evaluate(() => {
    const prototype = IDBDatabase.prototype as IDBDatabase & {
      __fixtureOriginalTransaction: IDBDatabase["transaction"];
    };
    prototype.transaction = prototype.__fixtureOriginalTransaction;
    delete (prototype as unknown as Record<string, unknown>)
      .__fixtureOriginalTransaction;
  });
  await page
    .getByRole("button", { name: "Restore pending inspection", exact: true })
    .click();
  await expect(page.getByLabel("Inspection save")).toHaveText("unknown");
  await page
    .getByRole("button", { name: "Retry original inspection", exact: true })
    .click();
  await expect(page.getByLabel("Inspection save")).toHaveText("succeeded");
  const evidence = await (await request.get("/__fixture/evidence")).json();
  const invocations = evidence.requests.filter(
    (row) =>
      row.route === "invoke" && JSON.parse(row.body).kind === "inspections",
  );
  expect(invocations).toHaveLength(2);
  expect(invocations[1].body).toBe(invocations[0].body);
});

test("public live revision invalidates arithmetic and ready export before recomputation", async ({
  page,
  request,
}) => {
  await page.goto("/?workspace=public");
  await expect(page.getByLabel("Public guide observation")).toHaveText("fresh");
  const card = page.getByRole("button", {
    name: "Electrical inspection",
    exact: true,
  });
  await card.click();
  await page
    .getByRole("button", {
      name: "Compute selected nominal values",
      exact: true,
    })
    .click();
  await expect(page.getByLabel("Selected nominal value sum")).toHaveText("230");
  await page
    .getByRole("button", {
      name: "Export selected public guide JSON",
      exact: true,
    })
    .click();
  await expect(page.getByLabel("Public export result")).toContainText(
    '"revision":1',
  );
  const updated = await request.post("/api/invoke", {
    headers: { authorization: "Bearer fixture-alice" },
    data: {
      kind: "maintenance-guides",
      id: "electrical",
      expected: 1,
      idempotency: "publish-updated-guide",
      operation: {
        type: "replace",
        input: {
          title: "Electrical inspection",
          nominal_voltage: 240,
          updated_at: "2026-10-08T13:00:00Z",
          internal_notes: "PRIVATE UPDATED NOTE",
        },
      },
    },
  });
  expect(updated.ok()).toBe(true);
  await expect(card).toContainText("Revision: 2");
  await expect(card).toContainText("Nominal value: 240");
  await expect(page.getByLabel("Selected nominal value sum")).toHaveText(
    "none",
  );
  await expect(page.getByLabel("Public export result")).toHaveText("none");
  await expect(
    page.getByRole("button", {
      name: "Download public guide JSON",
      exact: true,
    }),
  ).toHaveCount(0);
  await page
    .getByRole("button", {
      name: "Compute selected nominal values",
      exact: true,
    })
    .click();
  await expect(page.getByLabel("Selected nominal value sum")).toHaveText("240");
  await page
    .getByRole("button", {
      name: "Export selected public guide JSON",
      exact: true,
    })
    .click();
  await expect(page.getByLabel("Public export result")).toContainText(
    '"revision":2',
  );
  await expect(page.getByLabel("Public export result")).toContainText(
    '"nominalValueSum":240',
  );
  await expect(page.getByLabel("Public export result")).not.toContainText(
    "PRIVATE UPDATED NOTE",
  );
  const journal = await request.post("/api/journal", {
    headers: { authorization: "Bearer fixture-alice" },
    data: { kind: "maintenance-guides", after: null },
  });
  expect((await journal.json()).events).toHaveLength(2);
});

test("held public export cannot publish after selection changes in the same principal", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/?workspace=public");
  await expect(page.getByLabel("Public guide observation")).toHaveText("fresh");
  const card = page.getByRole("button", {
    name: "Electrical inspection",
    exact: true,
  });
  await card.click();
  await request.post("/__fixture/control", {
    data: { type: "hold", route: "read", kind: "maintenance-guides" },
  });
  await page
    .getByRole("button", {
      name: "Export selected public guide JSON",
      exact: true,
    })
    .click();
  await expect
    .poll(
      async () =>
        (await (await request.get("/__fixture/evidence")).json()).held,
    )
    .toBe(1);
  await card.click();
  await request.post("/__fixture/control", { data: { type: "release" } });
  await expect
    .poll(
      async () =>
        (await (await request.get("/__fixture/evidence")).json()).held,
    )
    .toBe(0);
  await expect(page.getByLabel("Current principal")).toHaveText("public-guest");
  await expect(page.getByLabel("Public export result")).toHaveText("none");
  await expect(
    page.getByRole("button", {
      name: "Download public guide JSON",
      exact: true,
    }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", {
      name: "Export selected public guide JSON",
      exact: true,
    }),
  ).toBeDisabled();
  expect(errors).toEqual([]);
});
