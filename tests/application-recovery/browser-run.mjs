import { readFileSync, writeFileSync, lstatSync } from "node:fs";
import { chromium } from "/root/ROM/studio/node_modules/@playwright/test/index.mjs";
import { awaitLoginDispatch } from "../identity/provider/login-dispatch.mjs";
import {
  protectedReadPath,
  readProtectedResource,
} from "../identity/provider/protected-read.mjs";
import { admitBrowserStorage } from "../identity/provider/browser-storage.mjs";
import { requireBrowserSocketBudget } from "../identity/provider/browser-paths.mjs";
const root = "/var/tmp/rom-010-authentik-20261007/run/volume",
  path = process.argv[2];
if (
  !path.startsWith(root + "/private/application-recovery-") ||
  lstatSync(path).size > 4096
)
  throw Error("private recovery browser configuration");
const c = JSON.parse(readFileSync(path));
requireBrowserSocketBudget(process.env.TMPDIR);
admitBrowserStorage(root, {
  home: process.env.HOME,
  temporary: process.env.TMPDIR,
});
const password = /^AUTHENTIK_BOOTSTRAP_PASSWORD=(.+)$/m.exec(
  readFileSync(c.environment, "utf8"),
)?.[1];
if (!password) throw Error("separately provisioned provider password required");
const origin = "https://127.0.0.1:44389";
let context,
  stage = "launch";
const record = {
  schema: 1,
  status: "starting",
  adapter: c.adapter,
  artifact_admission: false,
  original_consumer_acceptance: false,
};
try {
  context = await chromium.launchPersistentContext(
    process.env.HOME + "/browser-profile",
    { headless: true, executablePath: "/root/.nix-profile/bin/chromium" },
  );
  const page = await context.newPage();
  stage = "guest-before-login";
  const anonymousResponse = await context.request.get(
    origin + "/rom-studio/auth/session",
    { timeout: 5000 },
  );
  if (
    anonymousResponse.status() !== 200 ||
    (await anonymousResponse.json()).authenticated !== false
  )
    throw Error("fresh recovered Host unexpectedly authenticated guest");
  const guest = await context.request.post(origin + protectedReadPath, {
    headers: { origin },
    data: { kind: "fixture-documents", id: "private" },
    timeout: 5000,
  });
  if (guest.status() !== 401)
    throw Error("recovered guest accessed private Resource");
  record.guest_private_denial = 401;
  stage = "fresh-provider-login";
  const callback = page.waitForResponse(
    (r) => {
      const u = new URL(r.url());
      return (
        u.origin === origin &&
        u.pathname === "/rom-studio/auth/callback/authentik"
      );
    },
    { timeout: 20000 },
  );
  callback.catch(() => {});
  await page.goto(origin + "/rom-studio/auth/login/authentik", {
    timeout: 15000,
  });
  const input = page
      .locator("ak-stage-identification")
      .locator('input[name="uidField"]'),
    dispatch = await awaitLoginDispatch(
      input.waitFor({ state: "visible", timeout: 20000 }),
      callback,
    );
  if (dispatch.kind === "form") {
    await input.fill("akadmin");
    await page.getByRole("button", { name: /log in|continue/i }).click();
    const passwordStage = page.locator("ak-stage-password");
    await passwordStage.waitFor({ state: "visible", timeout: 10000 });
    await passwordStage.locator('input[name="password"]').fill(password);
    await passwordStage.locator('input[name="password"]').press("Enter");
  }
  const response =
    dispatch.kind === "callback" ? dispatch.response : await callback;
  if (response.status() !== 303) throw Error("recovered fresh login denied");
  record.callback_status = response.status();
  await page.waitForURL(
    (u) => u.origin === origin && u.pathname === "/rom-studio/",
    { timeout: 10000 },
  );
  stage = "current-identity";
  const sessionResponse = await context.request.get(
      origin + "/rom-studio/auth/session",
      { timeout: 5000 },
    ),
    session = await sessionResponse.json();
  if (
    sessionResponse.status() !== 200 ||
    session.authenticated !== true ||
    session.user_id !== "fixture-user" ||
    typeof session.csrf_token !== "string"
  )
    throw Error("restored identity resource mapping absent");
  record.session = {
    status: sessionResponse.status(),
    authenticated: true,
    expected_user: true,
  };
  stage = "current-authorized-resource";
  const value = await page.evaluate(readProtectedResource, {
    token: session.csrf_token,
    path: protectedReadPath,
  });
  if (value.status !== 200 || !value.protected_value)
    throw Error("restored Resource read denied");
  record.protected_read = {
    status: value.status,
    protected_value: value.protected_value,
  };
  stage = "authority-negative";
  const denied = await page.evaluate(readProtectedResource, {
    token: "invalid-fixture-csrf",
    path: protectedReadPath,
  });
  if (denied.status !== 403 || denied.protected_value)
    throw Error("restored authority bypass");
  record.csrf_denial = {
    status: denied.status,
    protected_value: denied.protected_value,
  };
  record.status = "passed";
  record.authenticated_checks_finished_unix_ms = Date.now();
} catch (error) {
  record.status = "failed";
  record.stage = stage;
  record.failure = error.message;
  process.exitCode = 1;
} finally {
  if (context) await context.close();
  writeFileSync(c.result, JSON.stringify(record, null, 2), {
    flag: "wx",
    mode: 0o600,
  });
}
