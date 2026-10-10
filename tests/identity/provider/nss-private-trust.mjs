// Private CA import/readback only. This does not start a browser or establish TLS acceptance.
import { readFileSync, writeFileSync, mkdirSync, statfsSync, lstatSync, realpathSync } from 'node:fs';
import { randomBytes, createHash, X509Certificate } from 'node:crypto';
import { OwnedLauncher } from './launch.mjs';
import { createProviderCgroup, admitOwnedWrapper, snapshotProviderCgroup, requireDrainedCgroup } from './cgroup.mjs';
import { admitBrowserStorage } from './browser-storage.mjs';
import { debianDataMember, admitToolTar, requirePackageDigest } from './debian.mjs';
const root = '/var/tmp/rom-010-authentik-20261007/run/volume';
const acquisition = `${root}/evidence/nss-tool-ee61adb2a4df6c0baecd5f6e/result.json`;
const toolRoot = '/var/tmp/rom-010-authentik-20261007/overlay/acquisition/nss-tools-ee61adb2a4df6c0baecd5f6e';
const openssl = '/nix/store/48p8b28v6a8vma8dz19gf75f0jsb073w-openssl-3.3.3-bin/bin/openssl';
const xz = '/nix/store/livin0fi0bzqnw9fqyx8acwbd4z4qrp9-xz-5.6.3-bin/bin/xz';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const packages = [
  { name: 'libnss3-tools', bytes: 1086124, sha256: '6504101943837ed2ee0089c9d4d72d69b96db0c51aa21249e0757855905af82e' },
  { name: 'libnss3', bytes: 1394236, sha256: '8d20d0754039e15e9bd2c8484e98dc9c2982907d4f20834da29a9501913d00b5' },
];
export async function probePrivateNss() {
  const acquired = JSON.parse(readFileSync(acquisition, 'utf8'));
  if (acquired.status !== 'acquired-dependencies-admitted' || acquired.directory !== toolRoot || realpathSync(root) !== root || lstatSync(root).dev === lstatSync('/var/tmp').dev) throw Error('private tool prerequisite');
  const available = statfsSync(root);
  if (Number(available.bavail) * Number(available.bsize) < 1024 ** 3) throw Error('private trust storage headroom');
  const unique = randomBytes(12).toString('hex'), evidence = `${root}/evidence/nss-private-trust-${unique}`, home = `${root}/private/nss-home-${unique}`;
  mkdirSync(evidence, { mode: 0o700 }); mkdirSync(home, { mode: 0o700 }); mkdirSync(`${home}/tmp`, { mode: 0o700 }); mkdirSync(`${home}/.pki`, { mode: 0o700 }); mkdirSync(`${home}/.pki/nssdb`, { mode: 0o700 });
  const group = createProviderCgroup(), launcher = new OwnedLauncher();
  const record = { schema: 'rom-private-nss-tool-probe-v1', status: 'starting', acquisition, acquisition_sha256: digest(readFileSync(acquisition)), home, processes: [], tls_admission: false };
  let stage = 'tool-revalidation';
  async function command(executable, args, outputBytes = 1024 ** 2, allowedExits = [0]) {
    const child = launcher.launch(executable, args, { timeoutMs: 30000, graceMs: 1000, drainMs: 1000, outputBytes, env: { HOME: home, TMPDIR: `${home}/tmp`, PATH: '/run/current-system/sw/bin:/usr/bin:/bin', LANG: 'C', TZ: 'UTC' }, admitIdentity: identity => admitOwnedWrapper(group, identity) });
    const chunks = []; let stdoutBytes = 0, stderrBytes = 0;
    child.child.stdout.on('data', bytes => { stdoutBytes += bytes.length; if (stdoutBytes <= outputBytes) chunks.push(bytes); }); child.child.stderr.on('data', bytes => { stderrBytes += bytes.length; });
    const target = await child.targetStarted, result = await child.closed; await child.physicalClose;
    record.processes.push({ executable, wrapper: child.identity, target, result, stdout_bytes: stdoutBytes, stderr_bytes: stderrBytes });
    if (!target || !allowedExits.includes(result.exit_code) || result.signal !== null || !result.drained || stdoutBytes > outputBytes) throw Error('private trust process failed');
    return Buffer.concat(chunks);
  }
  const nss = args => command(acquired.tool.loader.path, ['--library-path', acquired.tool.library_path, acquired.tool.path, ...args]);
  try {
    record.revalidated_files = [];
    for (const expected of packages) {
      const bytes = readFileSync(`${toolRoot}/${expected.name}.deb`); requirePackageDigest(bytes, expected);
      if (!debianDataMember(bytes).equals(readFileSync(`${toolRoot}/${expected.name}.data.tar.xz`))) throw Error('tool compressed member changed');
      const tar = await command(xz, ['--decompress', '--stdout', '--memlimit-decompress=256MiB', `${toolRoot}/${expected.name}.data.tar.xz`], 16 * 1024 ** 2);
      for (const [name, member] of admitToolTar(tar)) {
        const path = name === 'usr/bin/certutil' ? `${toolRoot}/bin/certutil` : /^usr\/lib\/x86_64-linux-gnu\/lib(?:nss3|nssutil3|smime3|softokn3|freebl3|freeblpriv3|nssckbi|ssl3|nssdbm3)\.so$/.test(name) ? `${toolRoot}/lib/${name.split('/').at(-1)}` : null;
        if (!path) continue;
        if (member.type !== '0' || !lstatSync(path).isFile() || realpathSync(path) !== path || !member.bytes.equals(readFileSync(path))) throw Error('tool projection changed');
        record.revalidated_files.push({ path, sha256: digest(member.bytes) });
      }
    }
    for (const dependency of [acquired.tool.loader, ...acquired.tool.dependencies]) if (realpathSync(dependency.path) !== dependency.realpath || digest(readFileSync(dependency.path)) !== dependency.sha256) throw Error('tool dependency changed');
    record.openssl = { path: openssl, realpath: realpathSync(openssl), sha256: digest(readFileSync(openssl)), version: (await command(openssl, ['version'])).toString('utf8').trim() };
    stage = 'synthetic-ca';
    const config = `${home}/ca.cnf`, key = `${home}/ca-key.pem`, certificate = `${home}/ca-cert.pem`;
    writeFileSync(config, '[req]\nprompt=no\ndistinguished_name=subject\nx509_extensions=extensions\n[subject]\nCN=ROM isolated identity fixture CA\n[extensions]\nbasicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\n', { flag: 'wx', mode: 0o600 });
    await command(openssl, ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-sha256', '-days', '1', '-config', config, '-keyout', key, '-out', certificate]);
    if (lstatSync(key).mode & 0o077) throw Error('synthetic private key permissions');
    const original = new X509Certificate(readFileSync(certificate)); if (!original.ca) throw Error('synthetic CA absent');
    record.ca = { certificate, der_sha256: digest(original.raw), valid_from: original.validFrom, valid_to: original.validTo };
    stage = 'private-nss-import';
    const database = `sql:${home}/.pki/nssdb`;
    await nss(['-N', '--empty-password', '-d', database]);
    await nss(['-A', '-d', database, '-n', 'ROM isolated identity fixture CA', '-t', 'C,,', '-i', certificate]);
    const listed = await nss(['-L', '-d', database, '-n', 'ROM isolated identity fixture CA', '-a']);
    const returned = new X509Certificate(listed);
    if (!original.raw.equals(returned.raw)) throw Error('NSS certificate readback mismatch');
    writeFileSync(`${evidence}/listed-ca.pem`, listed, { flag: 'wx', mode: 0o600 });
    record.storage = admitBrowserStorage(root, { home, temporary: `${home}/tmp`, profile: `${home}/.pki/nssdb` });
    record.status = 'private-ca-import-readback-green';
  } catch { record.status = 'private-nss-prerequisite-failed'; record.failure_stage = stage; process.exitCode = 1; }
  finally {
    record.drain = await launcher.drain(); record.group = snapshotProviderCgroup(group); requireDrainedCgroup(record.group.cgroup_events);
    writeFileSync(`${evidence}/result.json`, JSON.stringify(record, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
    console.log(JSON.stringify({ status: record.status, evidence, tls_admission: false }));
  }
}
