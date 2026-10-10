import { NativeTlsStageObserver } from './native-tls-stage-observer.mjs';
import { createHash } from 'node:crypto';
import { revalidateNativeTlsBrowserGraph } from './native-tls-browser-graph.mjs';
import { request } from 'node:https';
import { existsSync, mkdirSync, writeFileSync, readlinkSync, statfsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { OwnedLauncher, readProcessIdentity, sameProcessIdentity } from './launch.mjs';
import { createProviderCgroup, admitOwnedWrapper, snapshotProviderCgroup, requireDrainedCgroup } from './cgroup.mjs';
import { requireOwnedListener, requireVacantListener } from './network-admission.mjs';
import { readNativeTlsFile, readNativeTlsJson, readNativeTlsEnvironment, nativeTlsPrivatePath } from './native-tls-io.mjs';
import { nativeTlsAllocation, nativeTlsOwnedDirectoryAllocation, requireNativeTlsCombinedAllocation } from './native-tls-storage.mjs';
import { requireNativeTlsBrowserTemporary } from './native-tls-prepare.mjs';
import { admitBrowserStorage } from './browser-storage.mjs';
import { assertNativeTlsOutcome, requireNativeTlsProviderWindow } from './native-tls-profile.mjs';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const write = (path, value) => writeFileSync(path, JSON.stringify(value), { flag: 'wx', mode: 0o600 });
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

function fence(value) { if (hash(readNativeTlsFile(value.path, 128 * 1024 ** 2)) !== value.sha256) throw Error('native TLS input fence changed'); }
async function owned(port, identity, observer) {
  for (let i = 0; i < 40; i++) { observer.listenerAttempt(); try { requireOwnedListener(port, identity); observer.guard('listener', true); return; } catch { await pause(100); } }
  observer.guard('listener', false);
  throw Error('owned allocated listener startup failed');
}
function hostRequest(path, ca, cookie, method = 'GET', body, csrf) {
  return new Promise((resolve, reject) => {
    const headers = { ...(cookie ? { cookie } : {}), ...(body ? { 'content-type': 'application/json', origin: 'https://127.0.0.1:44389' } : {}), ...(csrf ? { 'x-rom-csrf': csrf } : {}) };
    const call = request({ hostname: '127.0.0.1', port: 44389, path, ca, method, headers, timeout: 5000 }, response => {
      const chunks = []; let length = 0;
      response.on('data', bytes => { length += bytes.length; if (length > 65536) { call.destroy(); reject(Error('host response bound')); } else chunks.push(bytes); });
      response.on('error', reject);
      response.on('end', () => resolve({ status: response.statusCode, cookies: response.headers['set-cookie'] ?? [], bytes: Buffer.concat(chunks) }));
    });
    call.on('error', reject); call.on('timeout', () => call.destroy(Error('host request deadline')));
    call.end(body ? JSON.stringify(body) : undefined);
  });
}

// Call only under an independently reviewed, explicit finite root execution lease.
export async function runNativeTlsCase(preparationPath, leasePath) {
  nativeTlsPrivatePath(preparationPath); nativeTlsPrivatePath(leasePath);
  const preparationBytes = readNativeTlsFile(preparationPath, 1048576, true), preparation = JSON.parse(preparationBytes);
  const lease = readNativeTlsJson(leasePath, 4096);
  if (Object.keys(lease).sort().join(',') !== 'expires_unix_ms,preparation_sha256,schema' || lease.schema !== 'rom-native-tls-finite-lease-v1' || lease.preparation_sha256 !== hash(preparationBytes) || !Number.isSafeInteger(lease.expires_unix_ms) || lease.expires_unix_ms < Date.now() + 210000 || lease.expires_unix_ms > Date.now() + 300000) throw Error('explicit reviewed finite lease required');
  if (preparation.schema !== 'rom-native-tls-preparation-v1' || preparation.status !== 'prepared-not-executed') throw Error('unused native TLS preparation required');
  const directory = nativeTlsPrivatePath(preparation.directory), resultPath = `${directory}/result.json`;
  if (existsSync(resultPath)) throw Error('fresh native TLS case output required');
  const fixedInputs = [...preparation.inputs, preparation.binary, preparation.host_configuration, ...preparation.native_sources, ...preparation.fixture_sources, ...preparation.browser_inputs, ...preparation.leaves.flatMap(leaf => [leaf.certificate, leaf.key])];
  fixedInputs.forEach(fence);
  revalidateNativeTlsBrowserGraph('/root/ROM/studio', preparation.browser_graph);
  requireNativeTlsBrowserTemporary(preparation.browser_temporary);
  const ready = readNativeTlsJson(preparation.provider_ready, 1048576, false);
  requireNativeTlsProviderWindow(ready.ready_deadline_unix_ms);
  requireOwnedListener(44390, ready.relay.target);
  for (const port of [44389, 44391, 44392, 44393]) requireVacantListener(port);
  const free = statfsSync(directory, { bigint: true }); if (free.bavail * free.bsize < 2n * 1024n ** 3n) throw Error('native TLS free-space admission');
  const record = { schema: 'rom-native-tls-case-v1', status: 'starting', preparation_sha256: hash(preparationBytes), selected_case: preparation.selected_case, processes: [], production_dependency_admission: false, artifact_admission: false, original_consumer_acceptance: false };
  const observer = new NativeTlsStageObserver();
  const hostGroup = createProviderCgroup(), browserGroup = createProviderCgroup(), hostLauncher = new OwnedLauncher(), browserLauncher = new OwnedLauncher();
  let host, proxy, browser;
  const proxyStop = `${directory}/proxy.stop`, proxyResult = `${directory}/proxy-result.json`, browserResult = `${directory}/browser-result.json`, handoffPath = `${directory}/handoff/code.json`;
  async function launch(launcher, group, executable, args, env, role, timeoutMs = 130000) {
    const child = launcher.launch(executable, args, { timeoutMs, graceMs: 1000, drainMs: 1000, outputBytes: 1048576, env, admitIdentity: identity => admitOwnedWrapper(group, identity) });
    child.child.stdout.on('data', bytes => observer.consume(role, 'stdout', bytes)); child.child.stderr.on('data', bytes => observer.consume(role, 'stderr', bytes));
    const identity = await child.targetStarted;
    const birthCurrent = Boolean(identity && sameProcessIdentity(identity, readProcessIdentity(identity.pid))); observer.guard('process-birth', birthCurrent);
    if (!birthCurrent) throw Error('fresh owned process birth required');
    const observation = { role, wrapper: child.identity, target: identity }; record.processes.push(observation);
    return { child, observation };
  }
  async function closeObservation(value) {
    if (!value) return;
    value.observation.result = await value.child.closed; await value.child.physicalClose;
    if (value.observation.result.exit_code !== 0 || !value.observation.result.drained) throw Error('owned child failed or did not drain');
  }
  try {
    const nss = readNativeTlsJson(preparation.nss_reference, 1048576, false);
    const toolInputs = [nss.tool, nss.tool.loader, ...nss.tool.dependencies]; toolInputs.forEach(fence); record.private_browser_trust_inputs = toolInputs.map(value => ({ path: value.path, sha256: value.sha256 }));
    const browserEnvironment = { HOME: `${directory}/home`, TMPDIR: preparation.browser_temporary.path, PATH: '/run/current-system/sw/bin:/usr/bin:/bin', LANG: 'C', TZ: 'UTC', PLAYWRIGHT_BROWSERS_PATH: '/root/.cache/ms-playwright', SSL_CERT_FILE: preparation.provider_ca, NODE_EXTRA_CA_CERTS: preparation.provider_ca };
    record.browser_storage = admitBrowserStorage('/var/tmp/rom-010-authentik-20261007/run/volume', { home: `${directory}/home`, temporary: preparation.browser_temporary.path });
    mkdirSync(`${directory}/home/.pki`, { mode: 0o700 }); mkdirSync(`${directory}/home/.pki/nssdb`, { mode: 0o700 });
    for (const args of [['-N', '--empty-password', '-d', `sql:${directory}/home/.pki/nssdb`], ['-A', '-d', `sql:${directory}/home/.pki/nssdb`, '-n', 'ROM native TLS private CA', '-t', 'C,,', '-i', preparation.provider_ca]]) {
      observer.enter(args[0] === '-N' ? 'browser-trust-initialize' : 'browser-trust-import');
      const value = await launch(browserLauncher, browserGroup, nss.tool.loader.path, ['--library-path', nss.tool.library_path, nss.tool.path, ...args], browserEnvironment, 'private-browser-trust', 30000); await closeObservation(value);
    }
    observer.enter('native-host-launch');
    host = await launch(hostLauncher, hostGroup, preparation.host_command.executable, preparation.host_command.args, preparation.host_command.env, 'native-host');
    observer.enter('native-host-executable');
    const executableMatches = readlinkSync(`/proc/${host.observation.target.pid}/exe`) === preparation.binary.path; observer.guard('executable', executableMatches);
    if (!executableMatches) throw Error('native target executable mismatch');
    observer.enter('native-host-environment');
    const actualEnvironment = readNativeTlsEnvironment(host.observation.target.pid);
    const sortedEnvironment = value => Object.entries(value).sort(([a], [b]) => a.localeCompare(b));
    const trustMatches = sameProcessIdentity(host.observation.target, readProcessIdentity(host.observation.target.pid)) && JSON.stringify(sortedEnvironment(actualEnvironment)) === JSON.stringify(sortedEnvironment(preparation.host_command.env)); observer.guard('trust-environment', trustMatches);
    if (!trustMatches) throw Error('native child trust environment or birth changed');
    record.child_trust_environment_names = Object.keys(preparation.host_command.env).sort();
    observer.enter('native-host-listener'); await owned(44391, host.observation.target, observer);
    const proxyConfiguration = `${directory}/proxy.json`;
    write(proxyConfiguration, { case: { adapter: preparation.selected_case.adapter, route: preparation.selected_case.route, certificate: preparation.selected_case.certificate }, trusted_leaf: { certificate: preparation.leaves[0].certificate.path, key: preparation.leaves[0].key.path }, negative_leaf: { certificate: preparation.leaves[1].certificate.path, key: preparation.leaves[1].key.path }, control_directory: `${directory}/handoff`, result: proxyResult, stop_file: proxyStop });
    observer.enter('proxy-launch');
    proxy = await launch(hostLauncher, hostGroup, process.execPath, [fileURLToPath(new URL('./native-tls-proxy-run.mjs', import.meta.url)), proxyConfiguration], { PATH: browserEnvironment.PATH, LANG: 'C' }, 'native-tls-proxy');
    for (const port of [44389, 44392]) { observer.enter('proxy-listeners'); await owned(port, proxy.observation.target, observer); }
    const browserConfiguration = `${directory}/browser.json`; write(browserConfiguration, { environment: preparation.environment, handoff: handoffPath, result: browserResult });
    requireNativeTlsBrowserTemporary(preparation.browser_temporary);
    observer.enter('browser-launch');
    browser = await launch(browserLauncher, browserGroup, process.execPath, [fileURLToPath(new URL('./native-tls-browser-run.mjs', import.meta.url)), browserConfiguration], browserEnvironment, 'original-code-browser', 60000);
    observer.enter('browser-handoff');
    await closeObservation(browser); await browserLauncher.drain();
    record.browser_group = snapshotProviderCgroup(browserGroup); requireDrainedCgroup(record.browser_group.cgroup_events);
    if (readNativeTlsJson(browserResult).status !== 'authorization-held') throw Error('original provider authorization code not held');
    const handoff = readNativeTlsJson(handoffPath, 16384), callback = new URL(handoff.callback);
    if (Object.keys(handoff).sort().join(',') !== 'callback,cookie' || callback.origin !== 'https://127.0.0.1:44389' || callback.pathname !== '/rom-studio/auth/callback/authentik' || callback.searchParams.getAll('code').length !== 1 || !callback.searchParams.get('code') || callback.searchParams.getAll('state').length !== 1 || !callback.searchParams.get('state') || typeof handoff.cookie !== 'string' || !/^rom_login=[A-Za-z0-9_-]{1,4096}$/.test(handoff.cookie)) throw Error('original private callback binding required');
    const proxyHeld = readNativeTlsJson(`${directory}/handoff/callback-held.json`, 16384);
    if (Object.keys(proxyHeld).sort().join(',') !== 'callback,cookie' || proxyHeld.callback !== handoff.callback || proxyHeld.cookie !== handoff.cookie) throw Error('proxy-held original callback binding changed');
    record.original_callback_proxy_held_before_native = true; observer.guard('authorization-handoff', true);
    write(`${directory}/handoff/arm.json`, { authorization_code_held: true, browser_drained: true });
    let acknowledged = false; for (let i = 0; i < 80; i++) { if (existsSync(`${directory}/handoff/armed.json`)) { const value = readNativeTlsJson(`${directory}/handoff/armed.json`); acknowledged = value.armed === true && value.route === preparation.selected_case.route; break; } await pause(25); }
    if (!acknowledged) throw Error('native acquisition handoff not armed');
    observer.enter('native-acquisition');
    const ca = readNativeTlsFile(preparation.provider_ca, 65536);
    const response = await hostRequest(callback.pathname + callback.search, ca, handoff.cookie);
    const sessionCookie = response.cookies.filter(value => value.startsWith('rom_session='));
    if (sessionCookie.length > 1) throw Error('ambiguous native session cookie');
    const cookie = sessionCookie[0]?.split(';')[0];
    const sessionResponse = await hostRequest('/rom-studio/auth/session', ca, cookie), session = JSON.parse(sessionResponse.bytes);
    const protectedResponse = await hostRequest('/rom-studio/api/read', ca, cookie, 'POST', { kind: 'fixture-documents', id: 'private' }, session.csrf_token);
    let protectedValue = false; try { protectedValue = JSON.parse(protectedResponse.bytes)?.value?.content === 'Synthetic protected fixture document'; } catch {}
    if (preparation.selected_case.certificate === 'trusted' && (session.user_id !== 'fixture-user' || typeof session.csrf_token !== 'string')) throw Error('actual linked session and CSRF binding required');
    observer.enter('native-outcome');
    record.host_observation = { authorization_code_held: true, child_trust_fenced: true, native_callback_status: response.status, authenticated: session.authenticated === true, protected_read_status: protectedResponse.status, protected_value: protectedValue };
    record.status = 'observed-pending-drain';
  } catch (error) { record.execution_failure = observer.snapshot(error); record.status = 'failed'; }
  finally {
    for (const path of [`${directory}/host.stop`, proxyStop]) if (!existsSync(path)) writeFileSync(path, 'stop\n', { flag: 'wx', mode: 0o600 });
    try {
      observer.enter('drain');
      record.browser_drain = await browserLauncher.drain(); record.host_drain = await hostLauncher.drain();
      await closeObservation(host); await closeObservation(proxy);
      record.host_group = snapshotProviderCgroup(hostGroup); requireDrainedCgroup(record.host_group.cgroup_events);
      record.browser_group = snapshotProviderCgroup(browserGroup); requireDrainedCgroup(record.browser_group.cgroup_events);
      for (const port of [44389, 44391, 44392, 44393]) requireVacantListener(port);
      fixedInputs.forEach(fence);
      revalidateNativeTlsBrowserGraph('/root/ROM/studio', preparation.browser_graph);
      requireNativeTlsBrowserTemporary(preparation.browser_temporary);
      const caseAllocation = nativeTlsAllocation(directory), temporaryAllocation = nativeTlsOwnedDirectoryAllocation(preparation.browser_temporary.path);
      record.allocation = { ...requireNativeTlsCombinedAllocation(caseAllocation, temporaryAllocation), case_directory: caseAllocation, browser_temporary: temporaryAllocation };
      if (record.status === 'observed-pending-drain') { record.observation = { ...record.host_observation, ...readNativeTlsJson(proxyResult) }; if (record.observation.failed || !record.observation.armed) throw Error('native TLS proxy evidence failed'); assertNativeTlsOutcome(preparation.selected_case, record.observation); observer.guard('native-outcome', true); record.status = 'passed'; }
    } catch (error) { record.cleanup_failure = observer.snapshot(error); record.status = 'failed'; }
    record.execution_observation = observer.snapshot();
    write(resultPath, record);
  }
  if (record.status !== 'passed') throw Error('native TLS case did not pass');
  return record;
}
