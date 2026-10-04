import { test, expect } from "@playwright/test";
import { startHost } from "./host-fixture.mjs";
for (const backend of ["sqlite", "redb"])
  test(`real ${backend} human auth and generic resources`, async ({ page }) => {
    const host = await startHost(backend);
    test.info().annotations.push({
      type: "retained-fixture",
      description: host.directory,
    });
    try {
      await page.goto(host.url);
      await page
        .getByRole("link", { name: "Sign in with Local fixture" })
        .click();
      await page.getByLabel("Fixture account").selectOption("alice");
      await page.getByRole("button", { name: "Sign in", exact: true }).click();
      await page.getByRole("button", { name: "Allow", exact: true }).click();
      await expect(
        page.getByRole("button", { name: "Sign out", exact: true }),
      ).toBeVisible();
      await page.getByRole("button", { name: "tasks", exact: true }).click();
      await expect(
        page.getByRole("button", { name: "Open task-a", exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: "Open task-a", exact: true })
        .click();
      await page
        .getByLabel("title mode", { exact: true })
        .selectOption("value");
      await page
        .getByLabel("title value", { exact: true })
        .fill("Confirmed once");
      let dropped = false;
      await page.route("**/api/invoke", async (route) => {
        if (!dropped) {
          dropped = true;
          await route.fetch();
          await route.abort("failed");
        } else await route.continue();
      });
      await page
        .getByRole("button", { name: "Apply patch", exact: true })
        .click();
      await expect(
        page.getByRole("button", { name: "Retry same mutation" }),
      ).toBeVisible();
      await page.getByRole("button", { name: "Retry same mutation" }).click();
      await expect(
        page.getByText("Confirmed once", { exact: true }).first(),
      ).toBeVisible();
      await expect(page.getByText("Revision 2", { exact: true })).toBeVisible();
      await page.unroute("**/api/invoke");
      await page.getByLabel("Query limit").fill("1");
      await page
        .getByRole("button", { name: "Apply query", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Next page", exact: true })
        .click();
      await expect(
        page.getByText("Moving page 2", { exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: "Previous page", exact: true })
        .click();
      await expect(
        page.getByText("Moving page 1", { exact: true }),
      ).toBeVisible();
      const initialLive = page.waitForResponse(
        (r) => r.url().endsWith("/api/live") && r.status() === 200,
      );
      await page
        .getByRole("button", { name: "Observe live query", exact: true })
        .click();
      await initialLive;
      await page.waitForTimeout(300);
      for (let lease = 0; lease < 3; lease++) {
        const reopened = page.waitForResponse(
          (r) => r.url().endsWith("/api/live") && r.status() === 200,
        );
        await host.control({ op: "advance", seconds: 31 });
        await reopened;
        await expect(
          page.getByRole("button", { name: "Stop live query", exact: true }),
        ).toBeVisible();
        await expect(
          page.getByRole("button", { name: "Open task-a", exact: true }),
        ).toBeVisible();
        await page.waitForTimeout(1500);
      }
      await page
        .getByRole("button", { name: "Stop live query", exact: true })
        .click();
      await page
        .getByRole("button", { name: "inventory", exact: true })
        .click();
      await expect(
        page.getByRole("button", { name: "Open inventory-a", exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: "maintenance-tickets", exact: true })
        .click();
      await expect(
        page.getByText("18446744073709551615", { exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: "Open ticket-a", exact: true })
        .click();
      await expect(
        page.getByLabel("opaque mode", { exact: true }),
      ).toBeDisabled();
      await expect(
        page.getByLabel("required_handle mode", { exact: true }),
      ).toBeDisabled();
      if (process.env.ROM_STUDIO_DEMO_RENDERER === "1") {
        await page
          .getByLabel("optional_code mode", { exact: true })
          .selectOption("value");
        await page
          .getByLabel("optional_code value", { exact: true })
          .fill("ticket-o2");
        await page
          .getByLabel("code_list mode", { exact: true })
          .selectOption("value");
        await page
          .getByLabel("code_list[0] value", { exact: true })
          .fill("ticket-l2");
        await page
          .getByLabel("code mode", { exact: true })
          .selectOption("value");
        await page.getByLabel("code value", { exact: true }).fill("ticket-b2");
        await page
          .getByRole("button", { name: "Apply patch", exact: true })
          .click();
        await expect(
          page.getByText("TICKET-B2", { exact: true }).first(),
        ).toBeVisible();
        await expect(
          page.getByText("TICKET-O2", { exact: true }).first(),
        ).toBeVisible();
        await expect(
          page.getByText("TICKET-L2", { exact: true }).first(),
        ).toBeVisible();
      } else
        await expect(
          page
            .getByText(
              "Custom editor unavailable for demo-ticket-code version 1. Existing values are preserved.",
            )
            .first(),
        ).toBeVisible();
      await expect(
        page
          .getByText(/Custom editor unavailable for demo-opaque-handle/)
          .first(),
      ).toBeVisible();
      await host.restart();
      await page.reload();
      await page
        .getByRole("link", { name: "Sign in with Local fixture" })
        .click();
      await expect(
        page.getByRole("button", { name: "Sign out", exact: true }),
      ).toBeVisible();
      await page.getByRole("button", { name: "tasks", exact: true }).click();
      await expect(
        page.getByText("Confirmed once", { exact: true }),
      ).toBeVisible();
      await page.getByRole("button", { name: "Work", exact: true }).click();
      await page
        .getByRole("button", { name: "Load work capabilities", exact: true })
        .click();
      await page
        .getByRole("button", { name: "List work", exact: true })
        .click();
      await expect(
        page.getByRole("heading", { name: "Work", exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: "Resources", exact: true })
        .click();
      await page.getByRole("button", { name: "tasks", exact: true }).click();
      const revokedStream = page.waitForResponse(
        (r) => r.url().endsWith("/api/live") && r.status() === 200,
      );
      await page
        .getByRole("button", { name: "Observe live query", exact: true })
        .click();
      await revokedStream;
      await host.control({
        op: "user-enabled",
        id: "alice-user",
        enabled: false,
      });
      await expect(
        page.getByText("Confirmed once", { exact: true }),
      ).toHaveCount(0);
      await expect(
        page.getByRole("link", { name: "Sign in with Local fixture" }),
      ).toBeVisible({ timeout: 20000 });
      await host.control({
        op: "user-enabled",
        id: "alice-user",
        enabled: true,
      });
      await page
        .getByRole("link", { name: "Sign in with Local fixture" })
        .click();
      await expect(
        page.getByRole("button", { name: "Sign out", exact: true }),
      ).toBeVisible();
      await page.getByRole("button", { name: "Sign out", exact: true }).click();
      await expect(
        page.getByRole("link", { name: "Sign in with Local fixture" }),
      ).toBeVisible();
    } finally {
      await host.close();
    }
  });
for (const backend of ["sqlite", "redb"])
  test(`real ${backend} attachment publication, lost response, download and detach`, async ({
    page,
  }) => {
    const host = await startHost(backend);
    test.info().annotations.push({
      type: "retained-fixture",
      description: host.directory,
    });
    try {
      await page.goto(host.url);
      await page
        .getByRole("link", { name: "Sign in with Local fixture" })
        .click();
      await page.getByRole("button", { name: "Sign in", exact: true }).click();
      await page.getByRole("button", { name: "Allow", exact: true }).click();
      await page
        .getByRole("button", { name: "Attachments", exact: true })
        .click();
      await page
        .getByLabel("Attachment Resource ID", { exact: true })
        .fill("folder/name");
      await page.getByLabel("Attachment file", { exact: true }).setInputFiles({
        name: "hello.txt",
        mimeType: "text/plain",
        buffer: Buffer.from("real attachment bytes"),
      });
      let dropped = false;
      await page.route("**/blobs/upload?id=*", async (route) => {
        if (!dropped) {
          dropped = true;
          await route.fetch();
          await route.abort("failed");
        } else await route.continue();
      });
      await page
        .getByRole("button", { name: "Upload attachment", exact: true })
        .click();
      await expect(
        page.getByRole("button", {
          name: "Retry same attachment",
          exact: true,
        }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: "Retry same attachment", exact: true })
        .click();
      await expect(
        page.getByRole("heading", {
          name: "Committed Resource folder/name",
          exact: true,
        }),
      ).toBeVisible();
      const row = page.getByRole("row").filter({
        has: page.getByRole("button", {
          name: "Open folder/name",
          exact: true,
        }),
      });
      await expect(
        row.getByRole("cell", { name: "2", exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: "Open folder/name", exact: true })
        .click();
      const downloading = page.waitForEvent("download");
      await page
        .getByRole("button", { name: "Download attachment", exact: true })
        .click();
      const download = await downloading;
      const stream = await download.createReadStream();
      const chunks = [];
      for await (const chunk of stream) chunks.push(chunk);
      expect(Buffer.concat(chunks).toString()).toBe("real attachment bytes");
      await page
        .getByLabel("Confirm detachment of folder/name", { exact: true })
        .check();
      await page
        .getByRole("button", { name: "Detach attachment", exact: true })
        .click();
      await expect(
        row.getByRole("cell", { name: "detached", exact: true }),
      ).toBeVisible();
      await page.unroute("**/blobs/upload?id=*");
    } finally {
      await host.close();
    }
  });

for (const backend of ["sqlite", "redb"])
  test(`real ${backend} OS SIGTERM drains accepted publication and OIDC work`, async ({
    page,
    browser,
  }) => {
    let arm = false,
      entered = false,
      release = () => {};
    const barrier = new Promise<void>((resolve) => {
      release = resolve;
    });
    const host = await startHost(backend, {
      beforeRequest: async (request: { url?: string }) => {
        if (arm && request.url?.split("?")[0] === "/token") {
          entered = true;
          await barrier;
        }
      },
    });
    test.info().annotations.push({
      type: "retained-fixture",
      description: host.directory,
    });
    const other = await browser.newContext();
    try {
      await page.goto(host.url);
      await page
        .getByRole("link", { name: "Sign in with Local fixture" })
        .click();
      await page.getByRole("button", { name: "Sign in", exact: true }).click();
      await page.getByRole("button", { name: "Allow", exact: true }).click();
      await page
        .getByRole("button", { name: "Attachments", exact: true })
        .click();
      await page
        .getByLabel("Attachment Resource ID", { exact: true })
        .fill("shutdown-file");
      await page.getByLabel("Attachment file", { exact: true }).setInputFiles({
        name: "held.txt",
        mimeType: "text/plain",
        buffer: Buffer.from("survives SIGTERM"),
      });
      await host.control({ op: "blob-pause" });
      await page
        .getByRole("button", { name: "Upload attachment", exact: true })
        .click();
      await expect
        .poll(
          async () =>
            (await host.control({ op: "blob-status" })).control_result.entered,
        )
        .toBe(true);
      await page.goto("about:blank");
      arm = true;
      const second = await other.newPage();
      await second.goto(host.url);
      await second
        .getByRole("link", { name: "Sign in with Local fixture" })
        .click();
      await second
        .getByRole("button", { name: "Sign in", exact: true })
        .click();
      const consent = second
        .getByRole("button", { name: "Allow", exact: true })
        .click({ noWaitAfter: true })
        .catch(() => {});
      await expect.poll(() => entered).toBe(true);
      await second.goto("about:blank");
      await consent;
      const exiting = host.terminate();
      expect(host.alive()).toBe(true);
      await host.control({ op: "blob-release" });
      expect(host.alive()).toBe(true);
      release();
      expect(await exiting).toEqual({ code: 0, signal: null });
      await host.restart();
      await page.goto(host.url);
      await page
        .getByRole("link", { name: "Sign in with Local fixture" })
        .click();
      if (
        await page
          .getByRole("button", { name: "Sign in", exact: true })
          .isVisible()
      )
        await page
          .getByRole("button", { name: "Sign in", exact: true })
          .click();
      if (
        await page
          .getByRole("button", { name: "Allow", exact: true })
          .isVisible()
      )
        await page.getByRole("button", { name: "Allow", exact: true }).click();
      await page
        .getByRole("button", { name: "Attachments", exact: true })
        .click();
      const row = page.getByRole("row").filter({
        has: page.getByRole("button", {
          name: "Open shutdown-file",
          exact: true,
        }),
      });
      await expect(
        row.getByRole("cell", { name: "ready", exact: true }),
      ).toBeVisible();
      await expect(
        row.getByRole("cell", { name: "2", exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: "Open shutdown-file", exact: true })
        .click();
      // Explicit negative probe changes real post-restart bytes after the host response.
      if (process.env.ROM_STUDIO_RECOVERY_CORRUPT_DOWNLOAD === "1") {
        await page.route(
          "**/blobs/attachment?id=shutdown-file",
          async (route) => {
            const actual = await route.fetch();
            const bytes = await actual.body();
            bytes[0] ^= 1;
            await route.fulfill({ response: actual, body: bytes });
          },
        );
      }
      const downloadEvent = page.waitForEvent("download");
      await page
        .getByRole("button", { name: "Download attachment", exact: true })
        .click();
      const recoveredDownload = await downloadEvent;
      const recoveredStream = await recoveredDownload.createReadStream();
      const recoveredChunks = [];
      for await (const chunk of recoveredStream) recoveredChunks.push(chunk);
      expect(Buffer.concat(recoveredChunks).toString()).toBe(
        "survives SIGTERM",
      );
    } finally {
      release();
      await other.close();
      await host.close();
    }
  });

for (const backend of ["sqlite", "redb"])
  test(`real ${backend} finite renderer query and stream measurement`, async ({
    page,
  }, testInfo) => {
    const host = await startHost(backend);
    const work: Record<string, number> = {};
    page.on("request", (request) => {
      const path = new URL(request.url()).pathname;
      if (path.includes("/rom-studio/api/")) work[path] = (work[path] ?? 0) + 1;
    });
    try {
      await page.goto(host.url);
      await page
        .getByRole("link", { name: "Sign in with Local fixture" })
        .click();
      await page.getByRole("button", { name: "Sign in", exact: true }).click();
      await page.getByRole("button", { name: "Allow", exact: true }).click();
      const started = await page.evaluate(() => performance.now());
      await page.getByRole("button", { name: "tasks", exact: true }).click();
      await expect(
        page.getByRole("button", { name: "Open task-c", exact: true }),
      ).toBeVisible();
      const rendered = await page.evaluate(
        () =>
          new Promise<number>((resolve) =>
            requestAnimationFrame(() => resolve(performance.now())),
          ),
      );
      const cells = await page.getByRole("cell").count();
      const live = page.waitForResponse(
        (response) =>
          response.url().endsWith("/api/live") && response.status() === 200,
      );
      const liveStarted = await page.evaluate(() => performance.now());
      await page
        .getByRole("button", { name: "Observe live query", exact: true })
        .click();
      await live;
      await expect(
        page.getByRole("button", { name: "Stop live query", exact: true }),
      ).toBeVisible();
      const liveOpened = await page.evaluate(
        () =>
          new Promise<number>((resolve) =>
            requestAnimationFrame(() => resolve(performance.now())),
          ),
      );
      await page
        .getByRole("button", { name: "Open task-a", exact: true })
        .click();
      await page
        .getByLabel("title mode", { exact: true })
        .selectOption("value");
      await page
        .getByLabel("title value", { exact: true })
        .fill("Measured live update");
      const mutationStarted = await page.evaluate(() => performance.now());
      await page
        .getByRole("button", { name: "Apply patch", exact: true })
        .click();
      await expect(
        page.getByText("Measured live update", { exact: true }).first(),
      ).toBeVisible();
      const mutated = await page.evaluate(
        () =>
          new Promise<number>((resolve) =>
            requestAnimationFrame(() => resolve(performance.now())),
          ),
      );
      const { writeFile } = await import("node:fs/promises");
      const report = {
        backend,
        engine: testInfo.project.name,
        dataset: { tasks: 3, inventory: 1, held_out_custom_resource: 1 },
        cells,
        measurements_ms: {
          query_to_visible_frame: rendered - started,
          stream_open_to_frame: liveOpened - liveStarted,
          mutation_to_visible_frame: mutated - mutationStarted,
        },
        http_requests: work,
        scope:
          "One local sample, real host and browser. Includes browser-driver scheduling, network, codecs and frame scheduling; not an isolated renderer or universal performance benchmark.",
      };
      await writeFile(
        `${host.directory}/cost-measurement.json`,
        JSON.stringify(report, null, 2),
      );
      console.log("ROM_FINITE_COST " + JSON.stringify(report));
    } finally {
      await host.close();
    }
  });

for (const backend of ["sqlite", "redb"])
  test(`real ${backend} shared session logout clears two tabs and closes delivery`, async ({
    page,
    context,
  }) => {
    const host = await startHost(backend);
    test.info().annotations.push({
      type: "retained-fixture",
      description: host.directory,
    });
    const second = await context.newPage();
    try {
      await page.goto(host.url);
      await page
        .getByRole("link", { name: "Sign in with Local fixture" })
        .click();
      await page.getByRole("button", { name: "Sign in", exact: true }).click();
      await page.getByRole("button", { name: "Allow", exact: true }).click();
      await page.getByRole("button", { name: "tasks", exact: true }).click();
      await expect(
        page.getByRole("button", { name: "Open task-a", exact: true }),
      ).toBeVisible();
      const opened = page.waitForResponse(
        (response) =>
          response.url().endsWith("/api/live") && response.status() === 200,
      );
      await page
        .getByRole("button", { name: "Observe live query", exact: true })
        .click();
      const delivery = await opened;
      await expect(
        page.getByRole("button", { name: "Stop live query", exact: true }),
      ).toBeVisible();
      await second.goto(host.url);
      await expect(
        second.getByRole("button", { name: "Sign out", exact: true }),
      ).toBeVisible();
      await second.getByRole("button", { name: "tasks", exact: true }).click();
      await expect(
        second.getByRole("button", { name: "Open task-a", exact: true }),
      ).toBeVisible();
      // Both pages share the same BrowserContext cookie/session, not copied Actor data.
      let terminal = "waiting";
      let terminalAt = 0;
      const observeTerminal = (
        request: import("@playwright/test").Request,
        result: string,
      ) => {
        if (request === delivery.request()) {
          terminal = result;
          terminalAt = Date.now();
        }
      };
      page.on("requestfinished", (request) =>
        observeTerminal(request, "finished"),
      );
      page.on("requestfailed", (request) => observeTerminal(request, "failed"));
      const logoutAt = Date.now();
      await second
        .getByRole("button", { name: "Sign out", exact: true })
        .click();
      await expect.poll(() => terminal, { timeout: 5000 }).not.toBe("waiting");
      for (const tab of [page, second]) {
        await expect(
          tab.getByRole("link", { name: "Sign in with Local fixture" }),
        ).toBeVisible({ timeout: 20000 });
        await expect(tab.getByRole("cell")).toHaveCount(0);
        await expect(
          tab.getByRole("button", { name: "Open task-a", exact: true }),
        ).toHaveCount(0);
      }
      console.log(
        "ROM_SHARED_SESSION " +
          JSON.stringify({
            backend,
            engine: test.info().project.name,
            delivery_terminal: terminal,
            delivery_terminal_after_logout_ms: terminalAt - logoutAt,
            both_tabs_cleared_after_logout_ms: Date.now() - logoutAt,
            scope:
              "Two pages share one real host session; browser request termination and current DOM clearing, not offline revocation.",
          }),
      );
      const current = await context.request.get(
        `${host.origin}/rom-studio/auth/session`,
      );
      expect(await current.json()).toEqual({
        authenticated: false,
        generation: "anonymous",
      });
    } finally {
      await second.close();
      await host.close();
    }
  });

for (const backend of ["sqlite", "redb"])
  test(`real ${backend} complete generic Task and Inventory browser workflows`, async ({
    page,
    context,
  }) => {
    const { humanLogin, accessible, createResource, updateResource } =
      await import("./resource-workflow.ts");
    page.setDefaultTimeout(10000);
    const host = await startHost(backend);
    const writer = await context.newPage();
    let observerQueries = 0;
    page.on("request", (r) => {
      if (new URL(r.url()).pathname.endsWith("/api/query")) observerQueries++;
    });
    try {
      await humanLogin(page, host.url);
      await writer.goto(host.url);
      await expect(
        writer.getByRole("button", { name: "Sign out", exact: true }),
      ).toBeVisible();
      for (const kind of ["tasks", "inventory"]) {
        const id = `browser-${kind}`;
        await createResource(page, kind, id);
        await page
          .getByRole("button", { name: `Open ${id}`, exact: true })
          .click();
        await expect(
          page.getByRole("region", { name: "Resource details" }),
        ).toContainText("Revision 1");
        if (kind === "inventory") {
          await expect(
            page.getByText("18446744073709551615", { exact: true }).first(),
          ).toBeVisible();
          await expect(
            page.getByRole("button", { name: "Run restock", exact: true }),
          ).toBeVisible();
        }
        const field = kind === "tasks" ? "title" : "quantity";
        const value = kind === "tasks" ? "Browser updated task" : "0";
        await updateResource(page, id, field, value);
        await page
          .getByLabel("Query field", { exact: true })
          .selectOption(kind === "tasks" ? "done" : "code");
        if (kind === "tasks")
          await page.getByLabel("Query value value", { exact: true }).uncheck();
        else
          await page
            .getByLabel("Query value value", { exact: true })
            .fill("BROWSER-STOCK");
        await page
          .getByLabel("Query sort field", { exact: true })
          .selectOption(field);
        await page
          .getByLabel("Query sort", { exact: true })
          .selectOption("asc");
        await page
          .getByRole("button", { name: "Apply query", exact: true })
          .click();
        await expect(
          page.getByRole("button", { name: `Open ${id}`, exact: true }),
        ).toBeVisible();
        const opened = page.waitForResponse(
          (r) => r.url().endsWith("/api/live") && r.status() === 200,
        );
        await page
          .getByRole("button", { name: "Observe live query", exact: true })
          .click();
        const response = await opened;
        let terminated = false;
        page.on("requestfailed", (r) => {
          if (r === response.request()) terminated = true;
        });
        page.on("requestfinished", (r) => {
          if (r === response.request()) terminated = true;
        });
        await writer.getByRole("button", { name: kind, exact: true }).click();
        const queriesBefore = observerQueries;
        if (kind === "tasks") {
          await writer
            .getByRole("button", { name: `Open ${id}`, exact: true })
            .click();
          await writer
            .getByRole("button", { name: "Run complete", exact: true })
            .click();
          await expect(
            page.getByRole("button", { name: `Open ${id}`, exact: true }),
          ).toHaveCount(0);
          await expect(
            page.getByRole("button", { name: "Open task-a", exact: true }),
          ).toBeVisible();
          expect(observerQueries).toBe(queriesBefore);
          await page
            .getByLabel("Query field", { exact: true })
            .selectOption("");
          await page
            .getByRole("button", { name: "Apply query", exact: true })
            .click();
        } else {
          const frames = [];
          for (let revision = 1; revision <= 3; revision++) {
            const started = await page.evaluate(() => performance.now());
            await updateResource(writer, id, "quantity", String(revision));
            await expect(
              page
                .getByRole("row")
                .filter({
                  has: page.getByRole("button", {
                    name: `Open ${id}`,
                    exact: true,
                  }),
                })
                .getByRole("cell")
                .nth(3),
            ).toHaveText(String(revision));
            await page.bringToFront();
            const visible = await page.evaluate(
              () =>
                new Promise<number>((resolve) =>
                  requestAnimationFrame(() => resolve(performance.now())),
                ),
            );
            await expect(page.getByRole("cell")).toHaveCount(5);
            frames.push(visible - started);
          }
          console.log(
            "ROM_FINITE_SNAPSHOTS " +
              JSON.stringify({
                backend,
                engine: test.info().project.name,
                snapshots: 3,
                observer_query_requests_during_snapshots:
                  observerQueries - queriesBefore,
                cells: 5,
                mutation_to_frame_ms: frames,
                scope:
                  "Real host sequential changes, browser-driver/network/frame-inclusive samples; DOM count, not heap GC.",
              }),
          );
          expect(observerQueries).toBe(queriesBefore);
          await writer
            .getByRole("button", { name: `Open ${id}`, exact: true })
            .click();
          await writer
            .getByLabel("input mode", { exact: true })
            .selectOption("value");
          await writer.getByLabel("input value", { exact: true }).fill("1");
          await writer
            .getByRole("button", { name: "Run restock", exact: true })
            .click();
          await expect(
            page.getByText("4", { exact: true }).first(),
          ).toBeVisible();
          await accessible(page, "connected live inventory details");
        }
        const stop = page.getByRole("button", {
          name: "Stop live query",
          exact: true,
        });
        if (await stop.count()) await stop.click();
        await expect.poll(() => terminated).toBe(true);
        await page
          .getByRole("button", { name: `Open ${id}`, exact: true })
          .click();
        await page
          .getByLabel(`Confirm deletion of ${id}`, { exact: true })
          .check();
        await page
          .getByRole("button", { name: "Delete Resource", exact: true })
          .click();
        await expect(
          page.getByRole("button", { name: `Open ${id}`, exact: true }),
        ).toHaveCount(0);
        await accessible(page, `${kind} after delete`);
      }
    } finally {
      await writer.close();
      await host.close();
    }
  });
