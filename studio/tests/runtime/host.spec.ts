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
