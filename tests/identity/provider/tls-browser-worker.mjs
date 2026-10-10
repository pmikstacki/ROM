import { readFileSync, writeFileSync, realpathSync } from 'node:fs';
import { createServer } from 'node:https';
import { createHash } from 'node:crypto';
import { chromium, webkit } from '/root/ROM/studio/node_modules/@playwright/test/index.mjs';
import { admitTlsObservation } from './tls-observation.mjs';
import { admitBrowserStorage } from './browser-storage.mjs';
import { requireBrowserSocketBudget } from './browser-paths.mjs';
import {snapshotLoadedCrypto,admitLoadedCrypto} from './crypto-wrapper.mjs';
const root = '/var/tmp/rom-010-authentik-20261007/run/volume';
const configPath = process.argv[2];
if (!configPath?.startsWith(`${root}/private/tls-`) || realpathSync(configPath) !== configPath) throw Error('owned TLS configuration required');
const configuration = JSON.parse(readFileSync(configPath, 'utf8'));
requireBrowserSocketBudget(process.env.TMPDIR);
let requests = 0, secureConnections = 0, tlsClientErrors = 0, context;
const server = createServer({ key: readFileSync(configuration.key), cert: readFileSync(configuration.certificate), minVersion: 'TLSv1.2' }, (_request, response) => {
  requests++; response.writeHead(200, { 'content-type': 'text/html', 'cache-control': 'no-store' }); response.end('<!doctype html><title>ROM isolated TLS trust fixture</title><p>Synthetic TLS trust fixture</p>');
});
server.on('secureConnection', () => secureConnections++); server.on('tlsClientError', () => tlsClientErrors++);
await new Promise((resolve, reject) => { server.once('error', reject); server.listen(44389, '127.0.0.1', resolve); });
const result = { schema: 'rom-private-browser-tls-prerequisite-v1', engine: configuration.engine, expected: configuration.expected, navigated: false, title: null, navigation_error: null, artifact_admission: false, provider_acceptance: false };
try {
  const crypto=configuration.crypto_wrapper;
  if(crypto){
    if(configuration.engine!=='webkit'||!/^\/var\/tmp\/rom-010-authentik-20261007\/overlay\/acquisition\/crypto-tools-[a-f0-9]{24}\/webkit-patched-crypto.sh$/.test(crypto.path)||realpathSync(crypto.path)!==crypto.path||createHash('sha256').update(readFileSync(crypto.path)).digest('hex')!==crypto.sha256)throw Error('private crypto wrapper changed');
    for(const file of [crypto.original,crypto.binary,...crypto.libraries])if(createHash('sha256').update(readFileSync(file.path)).digest('hex')!==file.sha256)throw Error('private crypto source changed');
  }
  const executable = configuration.engine === 'chromium' ? '/root/.nix-profile/bin/chromium' : crypto?.path??'/var/tmp/rom-studio-webkit-2359/pw_run.sh';
  result.executable = { path: executable, realpath: realpathSync(executable), sha256: createHash('sha256').update(readFileSync(executable)).digest('hex') };
  context = await (configuration.engine === 'chromium' ? chromium : webkit).launchPersistentContext(`${process.env.HOME}/browser-profile`, { headless: true, executablePath: executable });
  result.browser_version = context.browser()?.version() ?? 'persistent-context';
  const page = await context.newPage();
  try { await page.goto('https://127.0.0.1:44389/', { timeout: 15000, waitUntil: 'domcontentloaded' }); result.navigated = true; result.title = await page.title(); }
  catch (error) { result.navigation_error = String(error?.message ?? '').slice(0, 1024); }
  result.requests = requests; result.secure_connections = secureConnections; result.tls_client_errors = tlsClientErrors;
  if(crypto){const expected=crypto.libraries.map(file=>file.path);result.loaded_crypto=snapshotLoadedCrypto(expected,{requireMatch:false});admitLoadedCrypto(result.loaded_crypto.paths,expected);}
  admitTlsObservation(configuration.expected, result); result.status = 'passed';
} catch { result.status = 'prerequisite-failed'; process.exitCode = 1; }
finally {
  await context?.close(); server.closeAllConnections(); await new Promise(resolve => server.close(resolve));
  result.storage = admitBrowserStorage(root, { home: process.env.HOME, temporary: process.env.TMPDIR });
  writeFileSync(configuration.result, JSON.stringify(result, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
}
