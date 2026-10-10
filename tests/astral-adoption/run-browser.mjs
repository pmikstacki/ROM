// Actual loopback consumer trial. A separate owned launcher supplies the provider window.
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, readdirSync, lstatSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { isDeepStrictEqual } from 'node:util';
import { inventory } from '../../scripts/packages/astral-adoption.mjs';
import { command } from '../../scripts/packages/astral-owned-command.mjs';
import { OwnedLauncher, readProcessIdentity, sameProcessIdentity } from '../identity/provider/launch.mjs';
import { SyntheticProviderApi } from '../identity/provider/provider-api.mjs';
import { requireVacantListener } from '../identity/provider/network-admission.mjs';
import { withAstralRedirect } from './provider-redirect.mjs';
import { requireBrowserWindow, requireRecoveryResult } from './admission.mjs';
import { auditSourceInputs, finalObservation } from './source-fence.mjs';

const prefix = '/root/ROM/.worktrees/astral-010-public-adoption/web/.superpowers/rom-actual-recovery-';
const [root, providerId] = process.argv.slice(2);
if (!root?.startsWith(prefix) || !/^[A-Za-z0-9]{6}$/.test(root.slice(prefix.length)) ||
    !/^[a-f0-9]{32}$/.test(providerId)) throw Error('owned consumer/provider identifiers required');
const providerRoot = '/var/tmp/rom-010-authentik-20261007/run/volume';
const ready = JSON.parse(readFileSync(providerRoot + '/evidence/authentik-ready-' + providerId + '.json'));
const witness = JSON.parse(readFileSync(root + '/preparation.json'));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function sourceAudit() {
  auditSourceInputs(witness);
  if (hash(readFileSync(witness.binary)) !== witness.binary_sha256 ||
      hash(readFileSync(root + '/host.json')) !== witness.host_config_sha256 ||
      hash(readFileSync(root + '/recovery.spec.ts')) !== witness.generated_recovery_sha256 ||
      !isDeepStrictEqual(inventory(witness.assets), witness.asset_inventory)) {
    throw Error('actual consumer input changed');
  }
}
function socket() {
  return execFileSync('/run/current-system/sw/bin/ss', ['-ltnp', '( sport = :43902 )'],
    { encoding: 'utf8', timeout: 1000, maxBuffer: 16384 });
}
function privateBytes() {
  let bytes = 0, files = 0;
  function visit(path) {
    const stat = lstatSync(path);
    if (stat.isSymbolicLink()) throw Error('unexpected symbolic trial artifact');
    if (stat.isDirectory()) for (const name of readdirSync(path)) visit(path + '/' + name);
    else {
      if (!stat.isFile() || ++files > 1024) throw Error('trial artifact inventory exceeded');
      bytes += stat.size;
    }
  }
  visit(root);
  return bytes;
}
sourceAudit();
if (ready.unique !== providerId) throw Error('provider identity does not match its evidence path');
const initialRemaining = requireBrowserWindow(ready, witness, Date.now());
for (const role of ['postgres', 'server', 'worker']) {
  const born = ready.containers.find(value => value.role === role)?.observation?.birth;
  if (!born || !sameProcessIdentity(born, readProcessIdentity(born.pid))) throw Error('provider process identity changed');
}
// The relay is already owned; unused sibling endpoints must still be vacant.
for (const port of [44389, 44391, 44392, 44393]) requireVacantListener(port);
if (socket().trim().split('\n').length !== 1) throw Error('consumer endpoint occupied');
const environment = readFileSync(providerRoot + '/private/authentik-96b706d518977e471f458dfff7b9503a/authentik.env', 'utf8');
const api = new SyntheticProviderApi(environment, 'key');
const launcher = new OwnedLauncher({ maxProcesses: 1, outputBytes: 65536 });
const record = {
  schema: 'rom-astral-actual-browser-recovery-v1', engine: witness.engine,
  release_admitted: false, tls_acceptance: false, provider: providerId,
  initial_remaining_ms: initialRemaining, started_at: new Date().toISOString(),
  controls: [], status: 'running',
};
let host, target, monitor, hostOutput = '', interrupted = false;
const interrupt = () => { interrupted = true; void launcher.shutdown(); };
process.once('SIGINT', interrupt);
process.once('SIGTERM', interrupt);
const deadline = setTimeout(() => process.kill(process.pid, 'SIGTERM'), 300000);
try {
  await withAstralRedirect(api, 'http://127.0.0.1:43902/auth/callback/astral', async () => {
    if (interrupted) throw Error('consumer trial interrupted before launch');
    host = launcher.launch(witness.binary, [root + '/host.json'],
      { timeoutMs: 270000, graceMs: 1000, drainMs: 1000, outputBytes: 65536 });
    for (const stream of [host.child.stdout, host.child.stderr]) stream.on('data', chunk => {
      hostOutput = (hostOutput + chunk).slice(0, 65536);
    });
    target = await host.targetStarted;
    if (!target) throw Error('application target did not start');
    record.host_target = target;
    let healthy = false;
    for (let i = 0; i < 50; i++) {
      if (interrupted || !sameProcessIdentity(target, readProcessIdentity(target.pid))) throw Error('host unavailable before browser');
      try {
        const response = await fetch('http://127.0.0.1:43902/', { signal: AbortSignal.timeout(500), redirect: 'error' });
        if (response.status === 200) { healthy = true; break; }
      } catch { /* Each acquisition and the loop have finite bounds. */ }
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    if (!healthy || !socket().includes('pid=' + target.pid + ',') || !socket().includes('127.0.0.1:43902')) {
      throw Error('actual owned host not ready');
    }
    monitor = setInterval(() => {
      try {
        if (privateBytes() > 64 * 1024 * 1024 - 131072) {
          record.artifact_budget_exceeded = true;
          process.kill(process.pid, 'SIGTERM');
        }
      } catch {
        record.artifact_inventory_failed = true;
        process.kill(process.pid, 'SIGTERM');
      }
    }, 250);
    await command({ app: root, evidence: root, target: root + '/unused-target' },
      ['/root/ROM/.worktrees/astral-010-public-adoption/web/node_modules/@playwright/test/cli.js',
        'test', '--config', root + '/playwright.config.mjs'], 'browser', 240, 16 * 1024 * 1024, process.execPath);
    record.passed_cases = requireRecoveryResult(JSON.parse(readFileSync(root + '/results.json')));
    if (interrupted) throw Error('consumer trial interrupted');
  }, record.controls);
  record.status = 'passed';
} catch (error) {
  record.status = 'failed';
  record.error = error instanceof AggregateError ? 'trial and provider restoration failed' : error.message;
  process.exitCode = 1;
} finally {
  clearInterval(monitor);
  clearTimeout(deadline);
  try { record.host_shutdown = await launcher.shutdown(); }
  catch (error) { finalObservation(record, 'host_shutdown', () => { throw error; }); }
  if (host) await host.physicalClose;
  record.host_target_absent = finalObservation(record, 'host_absence', () => target ? readProcessIdentity(target.pid) === null : true);
  record.endpoint_vacant = finalObservation(record, 'endpoint_vacancy', () => socket().trim().split('\n').length === 1);
  record.provider_controls = api.records();
  record.input_unchanged = finalObservation(record, 'final_source_audit', () => { sourceAudit(); return true; });
  finalObservation(record, 'host_log_write', () => writeFileSync(root + '/host-output.private.log', hostOutput, { flag: 'wx', mode: 0o600 }));
  record.private_artifact_bytes_before_execution = finalObservation(record, 'artifact_inventory', privateBytes);
  record.reserved_execution_bytes = 65536;
  if (record.status !== 'passed' || !record.host_target_absent || !record.endpoint_vacant || !record.input_unchanged ||
      interrupted || record.artifact_budget_exceeded || record.artifact_inventory_failed ||
      record.private_artifact_bytes_before_execution === undefined ||
      record.private_artifact_bytes_before_execution + record.reserved_execution_bytes > 64 * 1024 * 1024) {
    record.status = 'failed'; process.exitCode = 1;
  }
  record.host_output_sha256 = hash(hostOutput);
  record.finished_at = new Date().toISOString();
  const encoded = JSON.stringify(record, null, 2) + '\n';
  finalObservation(record, 'execution_write', () => {
    if (Buffer.byteLength(encoded) > record.reserved_execution_bytes) throw Error('execution evidence budget exceeded');
    writeFileSync(root + '/execution.json', encoded, { flag: 'wx', mode: 0o600 });
  });
  if (record.final_failures?.some(value => value.stage === 'execution_write')) {
    process.exitCode = 1;
    finalObservation(record, 'fallback_execution_write', () => writeFileSync(
      '/root/ROM/.superpowers/rom-astral-final-failure-' + root.slice(prefix.length) + '.json',
      JSON.stringify(record, null, 2) + '\n', { flag: 'wx', mode: 0o600 }));
  }
  process.removeListener('SIGINT', interrupt);
  process.removeListener('SIGTERM', interrupt);
  console.log(JSON.stringify({ root, engine: record.engine, status: record.status,
    passed_cases: record.passed_cases, error: record.error, release_admitted: false }));
}
