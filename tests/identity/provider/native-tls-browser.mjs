// Capture an original provider authorization code. Drain before any native callback.
import { readNativeTlsBindingCookies } from './native-tls-callback-hold.mjs';
import { NativeTlsBrowserObservation } from './native-tls-browser-observation.mjs';
import { requireBrowserSocketBudget } from './browser-paths.mjs';
import { chromium } from '/root/ROM/studio/node_modules/@playwright/test/index.mjs';
import { existsSync, writeFileSync } from 'node:fs';
import { finishNativeAuthorizationSubmission } from './native-tls-handoff.mjs';
import { readNativeTlsFile, readNativeTlsJson, nativeTlsPrivatePath } from './native-tls-io.mjs';
const origin = 'https://127.0.0.1:44389';
export async function holdNativeTlsAuthorization(path) {
  nativeTlsPrivatePath(path);
  const config = readNativeTlsJson(path, 4096);
  if (Object.keys(config).sort().join(',') !== 'environment,handoff,result') throw Error('closed native browser configuration required');
  nativeTlsPrivatePath(config.handoff); nativeTlsPrivatePath(config.result);
  requireBrowserSocketBudget(process.env.TMPDIR);
  const environment = readNativeTlsFile(config.environment, 65536, true).toString();
  const password = /^AUTHENTIK_BOOTSTRAP_PASSWORD=(.+)$/m.exec(environment)?.[1];
  if (!password) throw Error('original private provider credential required');
  let context, page, held, cookies, captureCancelled = false; const observation = new NativeTlsBrowserObservation();
  try {
    context = await chromium.launchPersistentContext(`${process.env.HOME}/browser-profile`, { headless: true, executablePath: '/root/.nix-profile/bin/chromium' });
    // Chromium auto-continues redirect destinations: the owned TLS proxy holds it instead.
    if (!config.handoff.endsWith('/code.json')) throw Error('closed proxy callback marker required');
    const heldPath = config.handoff.slice(0, -9) + 'callback-held.json';
    const captureCallback = async () => {
      const deadline = Date.now() + 15000;
      while (!captureCancelled && Date.now() < deadline) {
        if (existsSync(heldPath)) { held = readNativeTlsJson(heldPath, 16384); return; }
        await new Promise(resolve => setTimeout(resolve, 25));
      }
      throw Error('authorization handoff deadline');
    };
    page = await context.newPage(); page.on('response', response => observation.response(response.url(), response.status())); page.on('framenavigated', frame => observation.location(frame.url())); observation.enter('navigation');
    await page.goto(`${origin}/rom-studio/auth/login/authentik`, { timeout: 15000 });
    const identification = page.locator('ak-stage-identification').locator('input[name="uidField"]');
    observation.enter('identification-wait'); await identification.waitFor({ state: 'visible', timeout: 15000 });
    observation.enter('identification-fill'); await identification.fill('akadmin'); observation.enter('identification-submit'); await page.getByRole('button', { name: /log in|continue/i }).click();
    const passwordStage = page.locator('ak-stage-password'); observation.enter('password-wait'); await passwordStage.waitFor({ state: 'visible', timeout: 10000 });
    observation.enter('password-fill'); await passwordStage.locator('input[name="password"]').fill(password);
    const capture = captureCallback(); capture.catch(() => {});
    let timer;
    const captured = Promise.race([capture, new Promise((_, reject) => { timer = setTimeout(() => reject(Error('authorization handoff deadline')), 15000); })]);
    observation.enter('authorization-submit');
    try { await finishNativeAuthorizationSubmission(passwordStage.locator('input[name="password"]').press('Enter'), captured, () => Boolean(held)); }
    finally { clearTimeout(timer); }
    cookies = await readNativeTlsBindingCookies(context);
    observation.enter('browser-drain'); await context.close(); context = null;
    observation.enter('binding-cookie'); const retained = cookies.filter(cookie => cookie.name === 'rom_login' && cookie.secure && cookie.httpOnly);
    if (retained.length !== 1 || retained[0].value.length > 4096 || !held || Object.keys(held).sort().join(',') !== 'callback,cookie' || held.cookie !== `${retained[0].name}=${retained[0].value}`) throw Error('original attempt binding cookie required');
    writeFileSync(config.handoff, JSON.stringify(held), { flag: 'wx', mode: 0o600 });
    writeFileSync(config.result, JSON.stringify({ status: 'authorization-held', browser_context_closed: true, original_code_held: true }), { flag: 'wx', mode: 0o600 });
  } catch (error) {
    const selectors = {}; if (page && !page.isClosed()) { observation.location(page.url()); selectors.frames = Math.min(32, page.frames().length); for (const [name, selector] of [['identification', 'ak-stage-identification input[name="uidField"]'], ['password', 'ak-stage-password input[name="password"]']]) { try { selectors[name] = Math.min(32, await page.locator(selector).count()); } catch {} } }
    writeFileSync(config.result, JSON.stringify(observation.failure(error, selectors)), { flag: 'wx', mode: 0o600 }); throw Error('native TLS authorization preparation failed');
  } finally { captureCancelled = true; await context?.close(); }
}
