// Current-source authoring reproduction only; not extracted artifact admission.
import { readFileSync, writeFileSync, mkdirSync, statfsSync, statSync } from 'node:fs';
import { createHash, randomBytes } from 'node:crypto';
import { OwnedLauncher } from './launch.mjs';
import { createProviderCgroup, admitOwnedWrapper, snapshotProviderCgroup, requireDrainedCgroup } from './cgroup.mjs';
import { reserveEvidence } from './admission.mjs';
import { requireOwnedListener, requireVacantListener } from './network-admission.mjs';
import { requireBrowserSocketBudget } from './browser-paths.mjs';
import { admitBrowserStorage } from './browser-storage.mjs';

const root = '/var/tmp/rom-010-authentik-20261007/run/volume';
const syntheticDirectory = `${root}/private/authentik-96b706d518977e471f458dfff7b9503a`;
const binary = '/root/ROM/target/debug/rom-identity-host-authoring';
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
function readJson(path) {
  if (statSync(path).size > 65536) throw Error('bounded fixture JSON exceeded');
  return JSON.parse(readFileSync(path, 'utf8'));
}
export async function runHostAuthoring(readyPath, buildIdentityPath) {
  if (!readyPath?.startsWith(`${root}/evidence/authentik-ready-`) || !readyPath.endsWith('.json')) throw Error('owned provider readiness record required');
  const ready = readJson(readyPath);
  requireOwnedListener(44390, ready.relay.target);
  if (!buildIdentityPath?.startsWith(`${root}/evidence/host-corrected-build-`) || !buildIdentityPath.endsWith('/build-identity.json')) throw Error('owned corrected build identity required');
  const build = readJson(buildIdentityPath);
  const drift = () => build.sources.filter(value => hash(value.path) !== value.sha256).map(value => value.path);
  if (build.binary.path !== binary || build.binary.sha256 !== hash(binary) || drift().length) throw Error('compiled authoring source/binary identity drift');
  const nonce = randomBytes(16).toString('hex');
  const evidence = reserveEvidence(`${root}/evidence/host-authoring-${nonce}`);
  const privateDirectory = reserveEvidence(`${root}/private/host-authoring-${nonce}`);
  const record = { schema: 'rom-host-original-provider-authoring-v1', lane: 'source-authoring-reproduction', tls_verified: false, artifact_admission: false, provider_ready_path: readyPath, binary: { path: binary, sha256: hash(binary) }, cases: [] };
  const verified = readJson(`${root}/evidence/sdk-authoring-20261007/actual-sdk-fresh-current-after-key-fix.json`);
  const verification = JSON.parse(verified.stdout);
  const flowPath = `${syntheticDirectory}/original-code-flow-attempt16.json`;
  if (verification.accepted !== true || verification.historical_replay !== false || verified.private_input_identity.path !== flowPath || verified.private_input_identity.sha256 !== hash(flowPath)) throw Error('previous owned current-time synthetic SDK proof/input mismatch');
  const flow = readJson(flowPath);
  // This bootstrap mapping comes from the previously verified, unchanged synthetic token.
  // It never accepts actor metadata from a Host request.
  const subject = JSON.parse(Buffer.from(flow.id_token.split('.')[1], 'base64url')).sub;
  if (typeof subject !== 'string' || !subject.length || subject.length > 2048) throw Error('verified synthetic subject missing');
  const client = readJson(`${syntheticDirectory}/oidc-client.json`);
  const discovery = readJson(`${root}/evidence/original-authentik-discovery.json`);
  const environment = readFileSync(`${syntheticDirectory}/authentik.env`, 'utf8');
  const token = /^AUTHENTIK_BOOTSTRAP_TOKEN=(.+)$/m.exec(environment)?.[1];
  if (!token) throw Error('private synthetic administration missing');
  const headers = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
  const get = await fetch('http://127.0.0.1:44390/api/v3/providers/oauth2/1/', { headers, redirect: 'error', signal: AbortSignal.timeout(3000) });
  if (!get.ok) throw Error('original provider configuration unavailable');
  const original = await get.json();
  const callback = 'http://127.0.0.1:44391/rom-studio/auth/callback/authentik';
  const redirects = original.redirect_uris;
  if (!redirects.some(value => value.matching_mode === 'strict' && value.url === callback)) redirects.push({ matching_mode: 'strict', url: callback });
  const patch = await fetch('http://127.0.0.1:44390/api/v3/providers/oauth2/1/', { method: 'PATCH', headers, body: JSON.stringify({ redirect_uris: redirects }), redirect: 'error', signal: AbortSignal.timeout(3000) });
  if (!patch.ok) throw Error('synthetic exact Host redirect rejected');
  record.synthetic_provider_redirect_patch_status = patch.status;
  let anyFailed = false;
  for (const adapter of ['sqlite', 'redb']) {
    requireOwnedListener(44390, ready.relay.target);
    requireVacantListener(44391);
    if (Number(statfsSync(root).bavail) * Number(statfsSync(root).bsize) < 512 * 1024 ** 2) throw Error('hard run volume headroom guard');
    const group = createProviderCgroup(), launcher = new OwnedLauncher();
    const adapterDirectory = `${privateDirectory}/${adapter}`;
    mkdirSync(adapterDirectory, { mode: 0o700 });
    const profileDirectory = `${adapterDirectory}/browser-home`;
    // Chromium creates Unix-domain sockets below TMPDIR; keep their absolute paths short.
    const temporaryDirectory = reserveEvidence(`${root}/t-${randomBytes(4).toString('hex')}`);
    requireBrowserSocketBudget(temporaryDirectory);
    mkdirSync(profileDirectory, { mode: 0o700 });
    const browserStorage = admitBrowserStorage(root, { home: profileDirectory, temporary: temporaryDirectory });
    const stopFile = `${adapterDirectory}/stop`, configPath = `${adapterDirectory}/host.json`;
    writeFileSync(configPath, JSON.stringify({ adapter, assets_directory: '/root/ROM/tests/identity/provider/host/assets', database: `${adapterDirectory}/database`, stop_file: stopFile, issuer: discovery.issuer, client_id: client.client_id, client_secret: client.client_secret, authorization_endpoint: discovery.authorization_endpoint, token_endpoint: discovery.token_endpoint, jwks_endpoint: discovery.jwks_uri, verified_synthetic_subject: subject }) + '\n', { flag: 'wx', mode: 0o600 });
    const resultPath = `${evidence}/${adapter}-browser.json`;
    const item = { adapter, group: null, host: null, browser: null, browser_storage_before: browserStorage, status: 'failed' };
    let host, hostTarget, hostOutput = '', hostStderr = '';
    try {
      host = launcher.launch(binary, [configPath], { timeoutMs: 130000, graceMs: 1000, drainMs: 1000, admitIdentity: identity => admitOwnedWrapper(group, identity) });
      hostTarget = await host.targetStarted;
      host.child.stdout.on('data', bytes => { hostOutput += bytes; });
      host.child.stderr.on('data', bytes => { if (hostStderr.length < 4096) hostStderr += bytes; });
      let admitted = false;
      for (let index = 0; index < 30; index++) {
        try { requireOwnedListener(44391, hostTarget); admitted = true; break; } catch { await new Promise(resolve => setTimeout(resolve, 100)); }
      }
      if (!admitted) throw Error('owned Host listener startup deadline');
      const browser = launcher.launch(process.execPath, ['/root/ROM/tests/identity/provider/host-browser-capture.mjs', syntheticDirectory, resultPath], { timeoutMs: 90000, graceMs: 1000, drainMs: 1000, env: { PATH: '/run/current-system/sw/bin:/usr/bin:/bin', LANG: 'C', TZ: 'UTC', HOME: profileDirectory, TMPDIR: temporaryDirectory, PLAYWRIGHT_BROWSERS_PATH: '/root/.cache/ms-playwright' }, admitIdentity: identity => admitOwnedWrapper(group, identity) });
      const browserTarget = await browser.targetStarted;
      let browserOutput = '', browserStderr = 0;
      browser.child.stdout.on('data', bytes => { browserOutput += bytes; }); browser.child.stderr.on('data', bytes => { browserStderr += bytes.length; });
      const browserResult = await browser.closed; await browser.physicalClose;
      item.browser = { wrapper: browser.identity, target: browserTarget, result: browserResult, stdout: browserOutput, stderr_bytes: browserStderr };
      writeFileSync(stopFile, 'stop\n', { flag: 'wx', mode: 0o600 });
      const hostResult = await host.closed; await host.physicalClose;
      item.host = { wrapper: host.identity, target: hostTarget, result: hostResult, stdout: hostOutput, stderr_bytes: Buffer.byteLength(hostStderr), startup_stage: /^host_fixture_stage=([a-z-]+)$/m.exec(hostStderr)?.[1] ?? null };
      item.status = browserResult.exit_code === 0 && hostResult.exit_code === 0 ? 'passed' : 'failed';
      if (item.status !== 'passed') anyFailed = true;
    } catch { anyFailed = true; item.status = 'fixture-failed'; }
    finally {
      if (host && !statSync(adapterDirectory).isDirectory()) throw Error('private adapter directory identity changed');
      try { writeFileSync(stopFile, 'stop\n', { flag: 'wx', mode: 0o600 }); } catch (error) { if (error.code !== 'EEXIST') throw error; }
      item.drain = await launcher.drain();
      if (host && !item.host) item.host = { wrapper: host.identity, target: hostTarget, result: await host.closed, stdout: hostOutput, stderr_bytes: Buffer.byteLength(hostStderr), startup_stage: /^host_fixture_stage=([a-z-]+)$/m.exec(hostStderr)?.[1] ?? null };
      item.group = snapshotProviderCgroup(group);
      requireDrainedCgroup(item.group.cgroup_events);
      record.cases.push(item);
    }
  }
  record.status = anyFailed ? 'failed' : 'passed';
  record.source_fence_after = { changed_paths: drift(), binary_unchanged: build.binary.sha256 === hash(binary) };
  if (record.source_fence_after.changed_paths.length || !record.source_fence_after.binary_unchanged) { record.status = 'source-drift'; anyFailed = true; }
  record.build_identity_path = buildIdentityPath;
  writeFileSync(`${evidence}/result.json`, JSON.stringify(record, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
  console.log(JSON.stringify({ status: record.status, evidence, cases: record.cases.map(value => ({ adapter: value.adapter, status: value.status })) }));
  if (anyFailed) process.exitCode = 1;
}
