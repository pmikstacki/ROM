// Real Authentik login for the isolated packaged application, not fixture API responses.
import { awaitLoginDispatch } from '../identity/provider/login-dispatch.mjs';
import { browserProfile, requireSessionCookie } from './browser-profile.mjs';

export async function loginAstral(page, { password, expectedUser, origin = 'http://127.0.0.1:43902' }) {
  const profile = browserProfile(origin);
  if (typeof password !== 'string' || password.length < 1 || password.length > 256 ||
      expectedUser !== 'astral-fixture-user') throw Error('private synthetic login input required');
  const callback = page.waitForResponse(response => {
    const url = new URL(response.url());
    return url.origin === origin && url.pathname === '/auth/callback/astral';
  }, { timeout: 30000 });
  callback.catch(() => {});
  await page.goto(origin + '/', { timeout: 15000 });
  await page.getByRole('link', { name: 'Wejdź do Astral Plane' }).click();
  const identification = page.locator('ak-stage-identification').locator('input[name="uidField"]');
  const dispatch = await awaitLoginDispatch(
    identification.waitFor({ state: 'visible', timeout: 20000 }), callback,
  );
  if (dispatch.kind === 'form') {
    await identification.fill('akadmin');
    await page.getByRole('button', { name: /log in|continue/i }).click();
    const stage = page.locator('ak-stage-password');
    await stage.waitFor({ state: 'visible', timeout: 10000 });
    await stage.locator('input[name="password"]').fill(password);
    await stage.locator('input[name="password"]').press('Enter');
  }
  const response = dispatch.kind === 'callback' ? dispatch.response : await callback;
  if (response.status() !== 303) throw Error('actual Astral callback rejected');
  await page.waitForURL(url => url.origin === origin && url.pathname === '/', { timeout: 10000 });
  const sessionResponse = await page.context().request.get(origin + '/auth/session', { timeout: 5000 });
  if (sessionResponse.status() !== 200) throw Error('actual Astral session unavailable');
  const session = await sessionResponse.json();
  if (session.authenticated !== true || session.user_id !== expectedUser ||
      typeof session.csrf_token !== 'string' || !/^[A-Za-z0-9_-]{1,512}$/.test(session.csrf_token)) {
    throw Error('actual linked Astral session required');
  }
  // Actual execution and its source witness establish acceptance, not profile selection.
  requireSessionCookie(await page.context().cookies(origin), profile);
  await page.getByRole('button', { name: 'Mój krąg', exact: true }).click();
  await page.getByRole('heading', { name: 'Twoi znajomi' }).waitFor({ state: 'visible', timeout: 10000 });
}
