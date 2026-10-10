// Historical synthetic-authoring runtime supplement only. No default daemon/store or build.
import { readFileSync, writeFileSync, mkdirSync, readdirSync, lstatSync, realpathSync, statfsSync } from 'node:fs';
import { createHash, randomBytes } from 'node:crypto';
import { OwnedLauncher } from './launch.mjs';
import { verifyGioClosure } from './gio-verification.mjs';
import { createProviderCgroup, admitOwnedWrapper, snapshotProviderCgroup, requireDrainedCgroup } from './cgroup.mjs';
const acquisition = '/var/tmp/rom-010-authentik-20261007/overlay/acquisition';
const run = '/var/tmp/rom-010-authentik-20261007/run/volume';
const metadata = `${run}/evidence/tls-trust-8319e733874e2f181e01d214/closure-metadata/result.json`;
const metadataHash = '659c24c6628e9c86ed0ba92597c391ff3fc5852c9eba53bb1819d2983d3e316f';
const nix = '/nix/store/dlzcxfyhwpvbwi907qsamgsqvi09gvd2-nix-2.24.14/bin/nix';
const modulePath = '/nix/store/pna9r6204grpyb4qsdfmdr9qjxsr6yhr-glib-networking-2.80.1';
const officialKey = 'cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function stored(root) {
  let bytes = 0, entries = 0;
  function visit(path) {
    if (++entries > 100000) throw Error('private store entry cap');
    const stat = lstatSync(path);
    if (stat.isDirectory()) for (const name of readdirSync(path)) visit(`${path}/${name}`);
    else if (stat.isFile() || stat.isSymbolicLink()) bytes += stat.size;
    else throw Error('private store file type');
  }
  visit(root); return { bytes, entries };
}
function totalStored() { return readdirSync(acquisition).filter(name => /^gio-tools-[a-f0-9]{24}$/.test(name)).reduce((total, name) => total + stored(`${acquisition}/${name}`).bytes, 0); }
export async function acquirePrivateGio() {
  if (hash(readFileSync(metadata)) !== metadataHash || realpathSync(acquisition) !== acquisition || lstatSync(acquisition).dev === lstatSync('/var/tmp').dev) throw Error('GIO acquisition profile changed');
  const expected = JSON.parse(readFileSync(metadata, 'utf8'));
  if (expected.records.length !== 35 || expected.compressed_bytes !== 21640360 || expected.nar_bytes !== 98544128) throw Error('GIO closure budget changed');
  const nonce = randomBytes(12).toString('hex'), directory = `${acquisition}/gio-tools-${nonce}`, evidence = `${run}/evidence/gio-tools-${nonce}`;
  mkdirSync(directory, { mode: 0o700 }); mkdirSync(evidence, { mode: 0o700 });
  for (const name of ['store-root', 'home', 'cache', 'tmp', 'configuration', 'modules']) mkdirSync(`${directory}/${name}`, { mode: 0o700 });
  const target = `local?root=${directory}/store-root&require-sigs=true`;
  const env = { HOME: `${directory}/home`, XDG_CACHE_HOME: `${directory}/cache`, TMPDIR: `${directory}/tmp`, NIX_CONF_DIR: `${directory}/configuration`, NIX_USER_CONF_FILES: '', NIX_SSL_CERT_FILE: '/etc/ssl/certs/ca-certificates.crt', PATH: '/run/current-system/sw/bin:/usr/bin:/bin', LANG: 'C', TZ: 'UTC', NIX_CONFIG: `experimental-features = nix-command\ntrusted-public-keys = ${officialKey}\nrequire-sigs = true\nsubstituters = https://cache.nixos.org\nconnect-timeout = 15\nstalled-download-timeout = 30\n` };
  const group = createProviderCgroup(), launcher = new OwnedLauncher();
  const record = { schema: 'rom-private-historical-gio-acquisition-v1', status: 'starting', metadata, metadata_sha256: metadataHash, directory, target_store: target, nix: { path: nix, realpath: realpathSync(nix), sha256: hash(readFileSync(nix)) }, official_key: officialKey, stored_byte_cap: 256 * 1024 ** 2, deadline_ms: 600000, processes: [], production_dependency_admission: false, advisory_gap: 'Pinned GnuTLS3.8.6 predates current official certificate-validation fixes; historical synthetic authoring only.' };
  let stage = 'private-store-info';
  const deadline = Date.now() + 600000;
  async function command(args, limit = 8 * 1024 ** 2) {
    const child = launcher.launch(nix, args, { timeoutMs: Math.max(1, Math.min(600000, deadline - Date.now())), graceMs: 1000, drainMs: 1000, outputBytes: limit, env, admitIdentity: identity => admitOwnedWrapper(group, identity) });
    const stdout = [], stderr = []; let stdoutBytes = 0, stderrBytes = 0, stopped = false;
    child.child.stdout.on('data', bytes => { stdoutBytes += bytes.length; if (stdoutBytes <= limit) stdout.push(bytes); }); child.child.stderr.on('data', bytes => { stderrBytes += bytes.length; if (stderrBytes <= limit) stderr.push(bytes); });
    const monitor = setInterval(() => {
      try { const fs = statfsSync(acquisition); if (totalStored() > record.stored_byte_cap || Number(fs.bavail) * Number(fs.bsize) < 1024 ** 3 || Date.now() > deadline) throw Error('GIO acquisition cap'); }
      catch { stopped = true; launcher.stop(child.identity.pid); }
    }, 500);
    let result, targetIdentity;
    try { targetIdentity = await child.targetStarted; result = await child.closed; await child.physicalClose; }
    finally { clearInterval(monitor); }
    const index = record.processes.length;
    writeFileSync(`${evidence}/process-${index}-stdout.log`, Buffer.concat(stdout), { flag: 'wx', mode: 0o600 }); writeFileSync(`${evidence}/process-${index}-stderr.log`, Buffer.concat(stderr), { flag: 'wx', mode: 0o600 });
    record.processes.push({ wrapper: child.identity, target: targetIdentity, result, args, stdout_bytes: stdoutBytes, stderr_bytes: stderrBytes, cap_stopped: stopped });
    if (!targetIdentity || result.exit_code !== 0 || !result.drained || result.signal !== null || stopped || stored(directory).bytes > record.stored_byte_cap) throw Error('private GIO process failed');
    return Buffer.concat(stdout);
  }
  try {
    record.store_info = JSON.parse((await command(['store', 'info', '--store', target, '--json'])).toString('utf8'));
    if (!lstatSync(`${directory}/store-root/nix/var/nix/db`).isDirectory()) throw Error('private Nix database missing');
    stage = 'signed-closure-copy';
    if (totalStored() + expected.nar_bytes + 32 * 1024 ** 2 > record.stored_byte_cap) throw Error('retained GIO acquisition budget');
    await command(['copy', '--from', 'https://cache.nixos.org', '--to', target, modulePath]);
    stage = 'exact-closure-verification';
    const actual = JSON.parse((await command(['path-info', '--store', target, '--recursive', '--json', modulePath])).toString('utf8'));
    record.verified_closure = verifyGioClosure(expected, actual);
    await command(['store', 'verify', '--store', target, '--recursive', '--sigs-needed', '1', modulePath]);
    stage = 'regular-tls-module-projection';
    const source = `${directory}/store-root${modulePath}/lib/gio/modules/libgiognutls.so`;
    if (!lstatSync(source).isFile() || realpathSync(source) !== source) throw Error('regular TLS module required');
    const projection = `${directory}/modules/libgiognutls.so`, bytes = readFileSync(source); writeFileSync(projection, bytes, { flag: 'wx', mode: 0o600 });
    record.module = { source, path: projection, sha256: hash(bytes), env_module_directory: `${directory}/modules`, private_ca_environment: 'NIX_SSL_CERT_FILE' };
    record.status = 'signed-historical-gio-closure-admitted';
  } catch { record.status = 'gio-prerequisite-failed'; record.failure_stage = stage; process.exitCode = 1; }
  finally {
    record.drain = await launcher.drain(); record.group = snapshotProviderCgroup(group); requireDrainedCgroup(record.group.cgroup_events); record.storage = stored(directory); record.total_retained_bytes = totalStored();
    writeFileSync(`${evidence}/result.json`, JSON.stringify(record, null, 2) + '\n', { flag: 'wx', mode: 0o600 }); console.log(JSON.stringify({ status: record.status, evidence, directory, stored_bytes: record.storage.bytes, production_dependency_admission: false }));
  }
}
