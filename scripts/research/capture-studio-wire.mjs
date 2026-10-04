// Capture only seeded Resource responses. Do not retain cookies or auth bodies.
import { mkdir, writeFile, readFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { createHash } from "node:crypto";
import { chromium } from "../../studio/node_modules/playwright-core/index.mjs";
import { startHost } from "../../studio/tests/runtime/host-fixture.mjs";

const output = resolve(
  process.argv[2] ?? "docs/research/evidence/rom-0.0.2/wire-parser",
);
const binary = process.env.ROM_STUDIO_DEMO_BINARY;
if (!binary)
  throw Error(
    "Set ROM_STUDIO_DEMO_BINARY to an accepted immutable fixture binary",
  );
await mkdir(output, { recursive: true });
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const browser = await chromium.launch({
  executablePath:
    process.env.ROM_CHROMIUM_PATH ?? "/root/.nix-profile/bin/chromium",
});
const manifest = {
  recordedAt: new Date().toISOString(),
  binarySha256: hash(await readFile(binary)),
  fixtures: [],
};
try {
  for (const backend of ["sqlite", "redb"]) {
    const host = await startHost(backend);
    const page = await browser.newPage();
    const pending = [];
    let sequence = 0;
    page.on("response", (response) => {
      const route = new URL(response.url()).pathname.split("/").at(-1);
      if (
        response.status() !== 200 ||
        !["discover", "read", "query"].includes(route)
      )
        return;
      const number = ++sequence;
      pending.push(
        (async () => {
          const bytes = await response.body();
          if (bytes.byteLength > 1048576)
            throw Error("fixture exceeds admission bound");
          const file = `${backend}-${number}-${route}.json`;
          await writeFile(join(output, file), bytes);
          manifest.fixtures.push({
            backend,
            route,
            file,
            bytes: bytes.byteLength,
            sha256: hash(bytes),
            request: JSON.parse(response.request().postData() ?? "{}"),
          });
        })(),
      );
    });
    try {
      await page.goto(host.url);
      await page
        .getByRole("link", { name: "Sign in with Local fixture" })
        .click();
      await page.getByLabel("Fixture account").selectOption("alice");
      await page.getByRole("button", { name: "Sign in", exact: true }).click();
      await page.getByRole("button", { name: "Allow", exact: true }).click();
      await page
        .getByRole("button", { name: "Sign out", exact: true })
        .waitFor();
      for (const [kind, id] of [
        ["tasks", "task-a"],
        ["inventory", "inventory-a"],
      ]) {
        await page.getByRole("button", { name: kind, exact: true }).click();
        await page
          .getByRole("button", { name: `Open ${id}`, exact: true })
          .click();
        await page.getByText("Revision 1", { exact: true }).waitFor();
      }
      await Promise.all(pending);
    } finally {
      await page.close();
      await host.close();
    }
  }
} finally {
  await browser.close();
}
manifest.fixtures.sort((a, b) => a.file.localeCompare(b.file));
if (
  !manifest.fixtures.some((f) => f.route === "discover") ||
  !manifest.fixtures.some((f) => f.route === "read") ||
  !manifest.fixtures.some((f) => f.route === "query")
)
  throw Error("required native response classes missing");
await writeFile(
  join(output, "capture.json"),
  JSON.stringify(manifest, null, 2) + "\n",
);
console.log(
  JSON.stringify({
    fixtures: manifest.fixtures.length,
    binarySha256: manifest.binarySha256,
  }),
);
