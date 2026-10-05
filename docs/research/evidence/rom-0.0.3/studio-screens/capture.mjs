// Synthetic local fixture screenshots, with immutable binary and asset provenance.
import { chromium, expect } from '../../../../../studio/node_modules/@playwright/test/index.mjs';
import { startHost } from '../../../../../studio/tests/runtime/host-fixture.mjs';
import { createHash } from 'node:crypto';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, join, resolve, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

const directory = dirname(fileURLToPath(import.meta.url));
const root = resolve(directory, '../../../../..');
const binary = process.env.ROM_STUDIO_DEMO_BINARY;
const assets = process.env.ROM_STUDIO_ASSETS;
if (!binary || !assets) throw Error('Set the verified binary and production assets explicitly.');
async function digest(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex');
}
async function tree(path) {
  const files = {};
  for (const entry of await readdir(path, { withFileTypes: true })) {
    const target = join(path, entry.name);
    if (entry.isDirectory()) Object.assign(files, await tree(target));
    else if (entry.isFile()) files[relative(root, target)] = await digest(target);
  }
  return files;
}
async function source() {
  const files = {};
  for (const path of ['studio/src', 'demo/src', 'demo/studio'])
    Object.assign(files, await tree(join(root, path)));
  for (const crate of await readdir(join(root, 'crates'), { withFileTypes: true })) {
    if (!crate.isDirectory()) continue;
    Object.assign(files, await tree(join(root, 'crates', crate.name, 'src')));
    files[`crates/${crate.name}/Cargo.toml`] = await digest(join(root, 'crates', crate.name, 'Cargo.toml'));
  }
  for (const path of ['Cargo.toml', 'Cargo.lock', 'studio/package.json', 'studio/package-lock.json', 'demo/Cargo.toml', 'studio/vite.config.ts'])
    files[path] = await digest(join(root, path));
  return files;
}
const before = await source();
const receipt = {
  timestamp: new Date().toISOString(), head: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
  capture_script: { path: relative(root, fileURLToPath(import.meta.url)), sha256: await digest(fileURLToPath(import.meta.url)) },
  binary: { path: binary, sha256: await digest(binary) },
  assets: { path: assets, sha256: await tree(assets) }, source_sha256: before,
  engine: 'chromium', viewport_desktop: { width: 1440, height: 1000 },
  viewport_mobile: { width: 390, height: 844 }, screenshots: [],
  scope: 'Actual local SQLite host and production assets, synthetic seeded demo data and human OIDC fixture. Development source evidence; release artifact and remote deployment acceptance are separate.',
};
const host = await startHost('sqlite');
receipt.retained_fixture = host.directory;
const browser = await chromium.launch({ executablePath: process.env.ROM_CHROMIUM_PATH ?? '/root/.nix-profile/bin/chromium' });
receipt.browser_version = browser.version();
try {
  const page = await browser.newPage({ viewport: receipt.viewport_desktop, colorScheme: 'light', locale: 'en-US', timezoneId: 'UTC' });
  await page.goto(host.url);
  await page.getByRole('link', { name: 'Sign in with Local fixture', exact: true }).click();
  await page.getByLabel('Fixture account').selectOption('alice');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('button', { name: 'Allow', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Sign out', exact: true })).toBeVisible();
  async function capture(file, screen) {
    await page.evaluate(async () => { await document.fonts.ready; await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))); });
    await page.screenshot({ path: join(directory, file), fullPage: false, animations: 'disabled' });
    receipt.screenshots.push({ file, screen, sha256: await digest(join(directory, file)) });
  }
  await page.getByRole('button', { name: 'Field showcase', exact: true }).click();
  await page.getByRole('button', { name: 'Open workshop-sample', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Resource details', exact: true })).toBeVisible();
  await expect(page.getByLabel('Scheduled date value', { exact: true })).toHaveValue('2026-10-05');
  await expect(page.getByRole('columnheader', { name: 'Resource', exact: true })).toBeVisible();
  await page.locator('[data-slot="table-container"]').evaluateAll(tables => tables.forEach(table => { table.scrollLeft = 0; }));
  await capture('field-showcase-desktop.png', 'Discovered semantic catalog and right Resource inspector with canonical controls and exact secondary ID.');
  await page.setViewportSize(receipt.viewport_mobile);
  await expect(page.getByLabel('Scheduled date value', { exact: true })).toBeVisible();
  await capture('field-showcase-mobile.png', 'Narrow Resource inspector for the same synthetic record.');
  await page.setViewportSize(receipt.viewport_desktop);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Settings groups', exact: true })).toBeVisible();
  await capture('settings-desktop.png', 'Authorized discovered Settings groups, without opening secret configuration values.');
  const listed = page.waitForResponse(response => response.url().endsWith('/api/work/list') && response.status() === 200);
  await page.getByRole('button', { name: 'Work', exact: true }).click();
  await listed;
  await page.setViewportSize(receipt.viewport_mobile);
  await expect(page.getByRole('heading', { name: 'Work', exact: true })).toBeVisible();
  await capture('work-mobile.png', 'Authorized Work list on a narrow viewport.');
  const after = await source();
  if (JSON.stringify(after) !== JSON.stringify(before) || await digest(binary) !== receipt.binary.sha256)
    throw Error('Source or immutable binary changed during capture; screenshots need a fresh receipt.');
  receipt.source_unchanged = true;
  receipt.result = 'passed';
  await writeFile(join(directory, 'source-facts.json'), JSON.stringify(receipt, null, 2) + '\n');
} finally {
  await browser.close();
  await host.close();
}
