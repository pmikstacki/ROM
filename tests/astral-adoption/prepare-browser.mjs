// Prepare a private real-host trial. This script does not start a provider or application.
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync, lstatSync } from 'node:fs';
import { inventory } from '../../scripts/packages/astral-adoption.mjs';

const capsule = '/root/ROM/.superpowers/rom-010-astral-packages-US3g7u';
const consumer = '/root/ROM/.worktrees/astral-010-public-adoption';
const provider = '/var/tmp/rom-010-authentik-20261007/run/volume/private/authentik-96b706d518977e471f458dfff7b9503a';
const proofPath = '/var/tmp/rom-010-authentik-20261007/run/volume/evidence/sdk-authoring-20261007/actual-sdk-fresh-current-after-key-fix.json';
const digest = value => createHash('sha256').update(value).digest('hex');
function privateInput(path, limit = 65536) {
  const stat = lstatSync(path);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > limit || (stat.mode & 0o777) !== 0o600) {
    throw Error('private regular bounded provider input required');
  }
  return readFileSync(path);
}
const engine = process.argv[2];
if (!['chromium', 'webkit'].includes(engine)) throw Error('closed browser engine required');
const acceptance = JSON.parse(readFileSync(capsule + '/adoption-evidence-retry-1/acceptance.json'));
const binary = capsule + '/adoption-target-retry-1/debug/astral-plane';
const binaryBytes = readFileSync(binary);
if (acceptance.completed !== true || digest(binaryBytes) !== acceptance.binarySha256 ||
    binaryBytes.length !== acceptance.binaryBytes) throw Error('actual adopted binary changed');
const proof = JSON.parse(readFileSync(proofPath));
const originalFlow = provider + '/original-code-flow-attempt16.json';
const originalBytes = privateInput(originalFlow);
if (proof.private_input_identity.path !== originalFlow ||
    proof.private_input_identity.sha256 !== digest(originalBytes) || proof.result.exit_code !== 0 ||
    JSON.parse(proof.stdout).accepted !== true) throw Error('verified synthetic identity input required');
const flow = JSON.parse(originalBytes);
const subject = JSON.parse(Buffer.from(flow.id_token.split('.')[1], 'base64url')).sub;
const client = JSON.parse(privateInput(provider + '/oidc-client.json'));
const environment = privateInput(provider + '/authentik.env').toString('utf8');
const password = /^AUTHENTIK_BOOTSTRAP_PASSWORD=(.+)$/m.exec(environment)?.[1];
const issuer = 'http://127.0.0.1:44390/application/o/rom-synthetic-identity/';
if (client.issuer !== issuer || flow.issuer !== issuer || typeof subject !== 'string' ||
    subject.length > 256 || !password || password.length > 256) throw Error('closed synthetic identity profile required');
const root = mkdtempSync(consumer + '/web/.superpowers/rom-actual-recovery-');
mkdirSync(root + '/assets', { mode: 0o700 });
// The host serves the already-built installed-package frontend; the witness binds its complete asset inventory.
const assets = consumer + '/web/dist';
const assetInventory = inventory(assets);
if (!assetInventory['index.html']) throw Error('installed frontend build missing');
writeFileSync(root + '/client-secret', client.client_secret, { mode: 0o600, flag: 'wx' });
writeFileSync(root + '/login.json', JSON.stringify({ password, expectedUser: 'astral-fixture-user' }), { mode: 0o600, flag: 'wx' });
const config = {
  origin: 'http://127.0.0.1:43902', listen: '127.0.0.1:43902', assets,
  database: root + '/application.sqlite', issuer, client_id: client.client_id,
  secret_file: root + '/client-secret', users: [{ id: 'astral-fixture-user', subject, name: 'ROM isolated consumer' }],
  authorization_endpoint: 'http://127.0.0.1:44390/application/o/authorize/',
  token_endpoint: 'http://127.0.0.1:44390/application/o/token/',
  jwks_endpoint: issuer + 'jwks/', interpretation: null, knowledge_enrichment: null,
  meta_deletion_secret_file: null, enrollment: null,
};
writeFileSync(root + '/host.json', JSON.stringify(config), { mode: 0o600, flag: 'wx' });
const source = readFileSync(consumer + '/web/tests/recovery.spec.ts', 'utf8');
const start = source.indexOf('async function login(page: Page) {');
const end = source.indexOf('\nasync function saveFriend', start);
if (start < 0 || end < 0 || source.indexOf('async function login(page: Page) {', start + 1) !== -1) {
  throw Error('unrecognized original recovery fixture boundary');
}
const login = `async function login(page: Page) {\n  await loginAstral(page, JSON.parse(readFileSync(${JSON.stringify(root + '/login.json')}, 'utf8')));\n}\n`;
const generated = `import { readFileSync } from 'node:fs';\nimport { loginAstral } from '/root/ROM/tests/astral-adoption/browser-login.mjs';\n` +
  source.slice(0, start) + login + source.slice(end);
writeFileSync(root + '/recovery.spec.ts', generated, { mode: 0o600, flag: 'wx' });
const executablePath = engine === 'chromium' ? '/root/.nix-profile/bin/chromium' : '/var/tmp/rom-studio-webkit-2359/pw_run.sh';
const browserConfig = {
  testDir: root, testMatch: 'recovery.spec.ts', timeout: 60000, workers: 1,
  use: { baseURL: config.origin, trace: 'off', screenshot: 'only-on-failure', video: 'off' },
  projects: [{ name: engine, use: { browserName: engine, launchOptions: { executablePath } } }],
  reporter: [['list'], ['json', { outputFile: root + '/results.json' }]], outputDir: root + '/test-results',
};
writeFileSync(root + '/playwright.config.mjs', 'export default ' + JSON.stringify(browserConfig) + ';\n', { mode: 0o600, flag: 'wx' });
const sourcePaths = [root + '/host.json', root + '/login.json', root + '/client-secret',
  root + '/recovery.spec.ts', root + '/playwright.config.mjs', consumer + '/web/tests/recovery.spec.ts',
  ...['prepare-browser', 'run-browser', 'admission', 'browser-login', 'browser-profile', 'provider-redirect', 'source-fence']
    .map(name => '/root/ROM/tests/astral-adoption/' + name + '.mjs'),
  ...['astral-adoption', 'astral-owned-command'].map(name => '/root/ROM/scripts/packages/' + name + '.mjs'),
  ...['launch', 'supervision', 'child-runner', 'provider-api', 'network-admission', 'login-dispatch']
    .map(name => '/root/ROM/tests/identity/provider/' + name + '.mjs')];
const witness = {
  schema: 'rom-astral-actual-recovery-preparation-v1', engine, root, binary,
  binary_sha256: digest(binaryBytes), assets, asset_inventory: assetInventory,
  recovery_source_sha256: digest(source), generated_recovery_sha256: digest(generated),
  source_inputs: sourcePaths.map(path => ({ path, sha256: digest(readFileSync(path)) })),
  host_config_sha256: digest(readFileSync(root + '/host.json')), identity_proof_sha256: digest(readFileSync(proofPath)),
  loopback_http: true, tls_acceptance: false, release_admitted: false,
  planned_browser_ms: 240000, required_provider_window_ms: 330000,
};
writeFileSync(root + '/preparation.json', JSON.stringify(witness, null, 2) + '\n', { mode: 0o600, flag: 'wx' });
console.log(JSON.stringify({ root, engine, release_admitted: false }));
