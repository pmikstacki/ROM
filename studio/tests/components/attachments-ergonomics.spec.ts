import { test, expect, type Page } from "@playwright/test";
import { createServer, type Server } from "node:http";

const receivers = new WeakMap<Page, Server>();
test.afterEach(async ({ page }) => {
  const server = receivers.get(page);
  if (server) {
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
  }
});

const kind = "fixture-files";
const ids = ["opaque-attachment-" + "x".repeat(180), "attachment-b"];
const descriptor = {
  kind,
  version: 1,
  fields: [
    { name: "title", shape: { type: "string" } },
    { name: "state", shape: { type: "string" } },
  ],
  actions: [],
  action_inputs: [],
  presentation: { label: "Attachments", title_field: "title" },
};
const row = (id: string) => ({
  key: { kind, id },
  revision: 1,
  value: {
    title: id === ids[0] ? "Annual report" : "Photo archive",
    state: "attached",
  },
});
async function fixture(page: Page, unavailable = false) {
  const reservations: Record<string, unknown>[] = [],
    uploads: { id: string; bytes: string }[] = [],
    detaches: string[] = [];
  let loseReserve = false,
    loseUpload = false;
  // WebKit does not expose binary postData through Playwright's route API.
  // A real local receiver verifies the bytes actually sent by each browser.
  const receiver = createServer(async (request, response) => {
    const cors = {
      "access-control-allow-origin": request.headers.origin ?? "*",
      "access-control-allow-credentials": "true",
      "access-control-allow-methods": "POST, OPTIONS",
      "access-control-allow-headers":
        request.headers["access-control-request-headers"] ??
        "content-type, x-rom-csrf",
    };
    if (request.method === "OPTIONS") {
      response.writeHead(204, cors);
      response.end();
      return;
    }
    const chunks: Buffer[] = [];
    for await (const chunk of request) chunks.push(Buffer.from(chunk));
    const id = new URL(request.url!, "http://fixture").searchParams.get("id")!;
    uploads.push({ id, bytes: Buffer.concat(chunks).toString() });
    if (loseUpload && uploads.length === 1) {
      request.socket.destroy();
      return;
    }
    response.writeHead(200, {
      "content-type": "application/json",
      ...cors,
    });
    response.end(JSON.stringify({ status: "attached", resource: row(id) }));
  });
  await new Promise<void>((resolve) =>
    receiver.listen(0, "127.0.0.1", resolve),
  );
  receivers.set(page, receiver);
  const address = receiver.address();
  if (!address || typeof address === "string")
    throw Error("Upload receiver failed to bind.");
  const receiverUrl = `http://127.0.0.1:${address.port}`;
  // Rewrite only the binary transport destination. The SDK and browser still
  // create and send the real Blob body; the server records actual HTTP bytes.
  await page.addInitScript(
    ({ receiver }) => {
      const original = globalThis.fetch.bind(globalThis);
      globalThis.fetch = (input, init) => {
        if (typeof input === "string" || input instanceof URL) {
          const url = new URL(input, location.href);
          if (url.pathname.endsWith("/blobs/upload"))
            return original(`${receiver}/upload${url.search}`, init);
        }
        return original(input, init);
      };
    },
    { receiver: receiverUrl },
  );
  await page.route("**/rom-studio/auth/session", (route) =>
    route.fulfill({
      json: {
        authenticated: true,
        generation: "attachment-session",
        csrf_token: "csrf",
        user_id: "human",
        expires_at: Math.floor(Date.now() / 1000) + 60,
      },
    }),
  );
  await page.route("**/rom-studio/api/**", (route) => {
    const endpoint = route.request().url().split("/").at(-1);
    return route.fulfill({
      json:
        endpoint === "discover"
          ? { version: 1, resources: [descriptor] }
          : ids.map(row),
    });
  });
  await page.route("**/rom-studio/blobs/**", async (route) => {
    const request = route.request(),
      endpoint = request.url().split("/").at(-1)!;
    if (endpoint === "capabilities")
      return unavailable
        ? route.fulfill({
            status: 403,
            json: { error: "denied", message: "Attachment access denied." },
          })
        : route.fulfill({
            json: {
              version: 1,
              resource_kind: kind,
              stores: ["local"],
              limits: { blob_bytes: 1024, chunk_bytes: 1024, chunks: 1 },
              operations: ["reserve", "upload", "download", "detach"],
            },
          });
    if (endpoint === "reserve") {
      const reservation = request.postDataJSON();
      reservations.push(reservation);
      if (loseReserve && reservations.length === 1)
        return route.abort("failed");
      return route.fulfill({
        json: { status: "reserved", resource: row(reservation.id) },
      });
    }
    if (endpoint === "detach") {
      const id = request.postDataJSON().id;
      detaches.push(id);
      return route.fulfill({ json: { status: "detached", resource: row(id) } });
    }
    return route.fulfill({ status: 404 });
  });
  await page.goto("./");
  await page
    .getByRole("button", { name: "Attachments", exact: true })
    .first()
    .click();
  return {
    reservations,
    uploads,
    detaches,
    loseNextReserve: () => {
      loseReserve = true;
    },
    loseNextUpload: () => {
      loseUpload = true;
    },
  };
}

test("attachment selection uses right inspector and exact confirmation binding", async ({
  page,
  browserName,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  const requests = await fixture(page);
  await page
    .getByRole("button", { name: `Open ${ids[0]}`, exact: true })
    .click();
  const details = page.getByRole("complementary", {
    name: "Attachment details",
    exact: true,
  });
  await expect(details).toBeVisible();
  await expect(
    details.getByRole("heading", { name: "Annual report", exact: true }),
  ).toBeVisible();
  await page
    .context()
    .grantPermissions(
      browserName === "chromium"
        ? ["clipboard-read", "clipboard-write"]
        : ["clipboard-read"],
    );
  await details
    .getByRole("button", { name: "Copy attachment ID", exact: true })
    .click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    ids[0],
  );
  await page
    .getByRole("checkbox", { name: `Confirm detachment of ${ids[0]}` })
    .check();
  await page
    .getByRole("button", { name: `Open ${ids[1]}`, exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Detach attachment", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("checkbox", { name: `Confirm detachment of ${ids[1]}` }),
  ).not.toBeChecked();
  await page
    .getByRole("checkbox", { name: `Confirm detachment of ${ids[1]}` })
    .check();
  await page
    .getByRole("button", { name: "Detach attachment", exact: true })
    .click();
  await expect.poll(() => requests.detaches).toEqual([ids[1]]);
});

test("lost reservation acknowledgement retains exact request and contents through inspector close", async ({
  page,
}) => {
  const requests = await fixture(page);
  requests.loseNextReserve();
  await page
    .getByLabel("Attachment Resource ID", { exact: true })
    .fill("exact-opaque-upload-id");
  await page.getByLabel("Attachment file", { exact: true }).setInputFiles({
    name: "report.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("unchanged\nfile bytes"),
  });
  await page
    .getByRole("button", { name: "Upload attachment", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Retry same attachment", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Open attachment details panel", exact: true })
    .click();
  await page
    .getByRole("button", {
      name: "Close attachment details panel",
      exact: true,
    })
    .click();
  await expect(
    page.getByLabel("Attachment Resource ID", { exact: true }),
  ).toHaveValue("exact-opaque-upload-id");
  await page
    .getByRole("button", { name: "Retry same attachment", exact: true })
    .click();
  await expect.poll(() => requests.uploads.length).toBe(1);
  expect(requests.reservations).toHaveLength(2);
  expect(requests.reservations[1]).toEqual(requests.reservations[0]);
  expect(requests.uploads[0]).toEqual({
    id: "exact-opaque-upload-id",
    bytes: "unchanged\nfile bytes",
  });
  await expect(
    page.getByRole("status").filter({ hasText: "Attachment committed" }),
  ).toBeVisible();
});

test("denied capability has explicit unavailable state and no successful empty list", async ({
  page,
}) => {
  await fixture(page, true);
  await expect(
    page.getByRole("heading", { name: "Attachments unavailable", exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("alert")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Upload attachment", exact: true }),
  ).not.toBeVisible();
  await expect(page.getByRole("table")).not.toBeVisible();
});

test("mobile attachment details contain focus and return to selected row", async ({
  page,
}) => {
  await fixture(page);
  await page.setViewportSize({ width: 390, height: 844 });
  const opener = page.getByRole("button", {
    name: `Open ${ids[0]}`,
    exact: true,
  });
  await opener.click();
  const dialog = page.getByRole("dialog", {
    name: "Attachment details",
    exact: true,
  });
  await expect(dialog).toBeVisible();
  await page.keyboard.press("Tab");
  expect(
    await dialog.evaluate((element) =>
      element.contains(document.activeElement),
    ),
  ).toBe(true);
  await page.keyboard.press("Escape");
  await expect(dialog).not.toBeVisible();
  await expect(opener).toBeFocused();
  await page.evaluate(() => {
    document.body.style.zoom = "2";
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("lost upload response retries identical bytes and prevents pending navigation", async ({
  page,
}) => {
  const requests = await fixture(page);
  requests.loseNextUpload();
  await page
    .getByLabel("Attachment Resource ID", { exact: true })
    .fill("frozen-upload");
  await page.getByLabel("Attachment file", { exact: true }).setInputFiles({
    name: "frozen.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("EXACT-BYTES"),
  });
  await page
    .getByRole("button", { name: "Upload attachment", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Retry same attachment", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Work", exact: true }),
  ).toHaveAttribute("aria-disabled", "true");
  await page
    .getByRole("button", { name: "Retry same attachment", exact: true })
    .click();
  await expect.poll(() => requests.uploads.length).toBe(2);
  expect(requests.reservations).toHaveLength(1);
  expect(requests.reservations[0].id).toBe("frozen-upload");
  expect(requests.uploads[1]).toEqual(requests.uploads[0]);
  await expect(
    page.getByRole("status").filter({ hasText: "Attachment committed" }),
  ).toBeVisible();
});

test("attachment list error stays distinct from empty success and can refresh", async ({
  page,
}) => {
  await fixture(page);
  let fail = true;
  await page.route("**/rom-studio/api/query", (route) =>
    route.fulfill(
      fail
        ? { status: 403, json: { error: "denied" } }
        : { json: ids.map(row) },
    ),
  );
  await page
    .getByRole("button", { name: "Refresh attachments", exact: true })
    .click();
  await expect(page.getByRole("alert")).toHaveText("denied");
  await expect(
    page.getByText("No authorized attachments are available.", { exact: true }),
  ).not.toBeVisible();
  fail = false;
  await page
    .getByRole("button", { name: "Refresh attachments", exact: true })
    .click();
  await expect(
    page.getByRole("cell", { name: "Annual report", exact: true }).first(),
  ).toBeVisible();
  await expect(page.getByRole("alert")).not.toBeVisible();
});
