// Public Host callback journey. Borrowed producer tooling is authoring-only.
import { readFileSync, writeFileSync } from 'node:fs';
import { requireBrowserSocketBudget } from './browser-paths.mjs';
import { admitBrowserStorage } from './browser-storage.mjs';

export async function runHostBrowser(privateDirectory, resultPath) {
  if (!privateDirectory?.startsWith('/var/tmp/rom-010-authentik-20261007/run/volume/private/') || !resultPath?.startsWith('/var/tmp/rom-010-authentik-20261007/run/volume/evidence/')) throw Error('owned authoring paths required');
  requireBrowserSocketBudget(process.env.TMPDIR);
  const runRoot = '/var/tmp/rom-010-authentik-20261007/run/volume';
  admitBrowserStorage(runRoot, { home: process.env.HOME, temporary: process.env.TMPDIR });
  const environment = readFileSync(`${privateDirectory}/authentik.env`, 'utf8');
  const password = /^AUTHENTIK_BOOTSTRAP_PASSWORD=(.+)$/m.exec(environment)?.[1];
  if (!password) throw Error('synthetic password missing');
  const { chromium } = await import('/root/ROM/studio/node_modules/@playwright/test/index.mjs');
  const tooling = JSON.parse(readFileSync('/root/ROM/studio/node_modules/@playwright/test/package.json', 'utf8'));
  let context, storage, stage = 'browser-launch', callbackStatus = null, authenticated = false;
  const origin = 'http://127.0.0.1:44391';
  try {
    context = await chromium.launchPersistentContext(`${process.env.HOME}/chromium-profile`, { headless: true, executablePath: '/root/.nix-profile/bin/chromium' });
    storage = admitBrowserStorage(runRoot, { home: process.env.HOME, temporary: process.env.TMPDIR, profile: `${process.env.HOME}/chromium-profile` });
    const page = await context.newPage();
    let acceptCallback;
    const callback = new Promise(resolve => { acceptCallback = resolve; });
    page.on('response', response => {
      const url = new URL(response.url());
      if (url.origin === origin && url.pathname === '/rom-studio/auth/callback/authentik') { callbackStatus = response.status(); acceptCallback(); }
    });
    stage = 'host-login';
    await page.goto(`${origin}/rom-studio/auth/login/authentik`, { timeout: 15000 });
    stage = 'identification';
    await page.locator('ak-stage-identification').locator('input[name="uidField"]').fill('akadmin', { timeout: 10000 });
    await page.getByRole('button', { name: /log in|continue/i }).click();
    stage = 'password';
    const passwordStage = page.locator('ak-stage-password');
    await passwordStage.waitFor({ state: 'visible', timeout: 10000 });
    await passwordStage.locator('input[name="password"]').fill(password);
    await passwordStage.locator('input[name="password"]').press('Enter');
    stage = 'host-callback';
    await Promise.race([callback, new Promise((_, reject) => setTimeout(() => reject(Error('callback deadline')), 15000))]);
    stage = 'current-session';
    const response = await context.request.get(`${origin}/rom-studio/auth/session`, { timeout: 5000 });
    if (response.status() !== 200) throw Error('session response failed');
    const session = await response.json();
    authenticated = session.authenticated === true;
    if (!authenticated) throw Error('original Host callback did not establish a session');
  } catch (error) {
    if (stage === 'browser-launch') writeFileSync(`${process.env.HOME}/browser-launch-error.txt`, String(error).slice(0, 8192), { flag: 'wx', mode: 0o600 });
    process.exitCode = 1;
  }
  finally {
    await context?.close();
    const result = { schema: 'rom-original-provider-host-authoring-v1', status: authenticated ? 'passed' : 'failed', stage, engine: 'chromium', callback_http_status: callbackStatus, authenticated, tooling: { mode: 'borrowed-producer-authoring-only', version: tooling.version, path: '/root/ROM/studio/node_modules/@playwright/test/index.mjs', executable: '/root/.nix-profile/bin/chromium' }, browser_storage: storage ?? null, tls_verified: false, artifact_admission: false };
    writeFileSync(resultPath, JSON.stringify(result, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
    console.log(JSON.stringify({ status: result.status, stage, callback_http_status: callbackStatus, authenticated, engine: result.engine, tls_verified: false, artifact_admission: false }));
  }
}
