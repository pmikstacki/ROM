// Coordinator-admitted authoring tool acquisition. No global package or trust installation.
import { readFileSync, writeFileSync, mkdirSync, readdirSync, lstatSync, statfsSync, realpathSync } from 'node:fs';
import { randomBytes, createHash } from 'node:crypto';
import { OwnedLauncher } from './launch.mjs';
import { createProviderCgroup, admitOwnedWrapper, snapshotProviderCgroup, requireDrainedCgroup } from './cgroup.mjs';
import { debianDataMember, admitToolTar, requirePackageDigest } from './debian.mjs';

const MIB = 1024 ** 2;
const acquisitionRoot = '/var/tmp/rom-010-authentik-20261007/overlay/acquisition';
const runRoot = '/var/tmp/rom-010-authentik-20261007/run/volume';
const loader = '/nix/store/5m9amsvvh2z8sl7jrnc87hzy21glw6k1-glibc-2.40-66/lib/ld-linux-x86-64.so.2';
const nspr = '/nix/store/l4n6x5rca1v0vglag60dl8351yzj7pn5-nspr-4.36/lib';
const sqlite = '/nix/store/v9smapvfv1z340qs3p7xbw6zb6zplfcf-sqlite-3.46.1/lib';
const glibc = '/nix/store/5m9amsvvh2z8sl7jrnc87hzy21glw6k1-glibc-2.40-66/lib';
const xz = '/nix/store/livin0fi0bzqnw9fqyx8acwbd4z4qrp9-xz-5.6.3-bin/bin/xz';
const packages = [
  { name: 'libnss3-tools', bytes: 1086124, sha256: '6504101943837ed2ee0089c9d4d72d69b96db0c51aa21249e0757855905af82e' },
  { name: 'libnss3', bytes: 1394236, sha256: '8d20d0754039e15e9bd2c8484e98dc9c2982907d4f20834da29a9501913d00b5' },
];
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function storedBytes(root) {
  let bytes = 0, count = 0;
  function visit(path) {
    if (++count > 2048) throw Error('tool storage entry cap');
    const stat = lstatSync(path);
    if (stat.isSymbolicLink()) throw Error('tool storage link');
    if (stat.isDirectory()) for (const name of readdirSync(path)) visit(`${path}/${name}`);
    else if (stat.isFile()) bytes += stat.size;
    else throw Error('tool storage file type');
  }
  visit(root); return bytes;
}
function totalToolBytes() {
  return readdirSync(acquisitionRoot).filter(name => /^nss-tools-[a-f0-9]{24}$/.test(name)).reduce((total, name) => total + storedBytes(`${acquisitionRoot}/${name}`), 0);
}
async function download(url, limit, deadline) {
  if (!url.startsWith('https://deb.debian.org/debian/pool/main/n/nss/') && !url.startsWith('https://metadata.ftp-master.debian.org/changelogs/main/n/nss/') && url !== 'https://security-tracker.debian.org/tracker/source-package/nss') throw Error('tool origin not approved');
  const response = await fetch(url, { redirect: 'error', signal: AbortSignal.timeout(Math.min(30000, Math.max(1, deadline - Date.now()))) });
  if (!response.ok) throw Error('tool source response rejected');
  let bytes = 0; const chunks = [];
  for await (const chunk of response.body) {
    bytes += chunk.length;
    const fs = statfsSync(acquisitionRoot);
    if (bytes > limit || Date.now() > deadline || Number(fs.bavail) * Number(fs.bsize) < 1024 * MIB) throw Error('tool acquisition limit');
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks);
}
export async function acquireNss() {
  if (realpathSync(acquisitionRoot) !== acquisitionRoot || lstatSync(acquisitionRoot).dev === lstatSync('/var/tmp').dev) throw Error('hard acquisition filesystem unavailable');
  const unique = randomBytes(12).toString('hex'), directory = `${acquisitionRoot}/nss-tools-${unique}`;
  mkdirSync(directory, { mode: 0o700 }); mkdirSync(`${directory}/bin`, { mode: 0o700 }); mkdirSync(`${directory}/lib`, { mode: 0o700 });
  const evidence = `${runRoot}/evidence/nss-tool-${unique}`; mkdirSync(evidence, { mode: 0o700 });
  const group = createProviderCgroup(), launcher = new OwnedLauncher();
  const record = { schema: 'rom-isolated-nss-tool-authoring-v1', status: 'starting', directory, version: '2:3.110-1+deb13u4', platform: 'linux/amd64', source_package: 'NSS 3.110 plus Debian deb13u4', licenses: ['MPL-2.0', 'Zlib', 'BSD-3-Clause', 'public-domain'], tool_storage_cap: 32 * MIB, acquisition_deadline_ms: 600000, packages: [], processes: [], tls_admission: false };
  const deadline = Date.now() + 600000;
  let stage = 'metadata';
  const save = (path, bytes, executable = false) => {
    if (totalToolBytes() + bytes.length > 32 * MIB || Date.now() > deadline) throw Error('tool total cap exceeded');
    writeFileSync(path, bytes, { flag: 'wx', mode: executable ? 0o700 : 0o600 });
  };
  async function command(executable, args, limit = 16 * MIB) {
    const child = launcher.launch(executable, args, { timeoutMs: 30000, graceMs: 1000, drainMs: 1000, outputBytes: limit, admitIdentity: identity => admitOwnedWrapper(group, identity) });
    let count = 0, stderrBytes = 0; const chunks = [];
    child.child.stdout.on('data', bytes => { count += bytes.length; if (count <= limit) chunks.push(Buffer.from(bytes)); });
    child.child.stderr.on('data', bytes => { stderrBytes += bytes.length; });
    const target = await child.targetStarted, result = await child.closed; await child.physicalClose;
    record.processes.push({ executable, wrapper: child.identity, target, result, stdout_bytes: count, stderr_bytes: stderrBytes });
    if (!target || result.exit_code !== 0 || result.signal !== null || !result.drained || count > limit) throw Error('tool process prerequisite failed');
    return Buffer.concat(chunks);
  }
  try {
    for (const [name, url] of [
      ['copyright', 'https://metadata.ftp-master.debian.org/changelogs/main/n/nss/nss_3.110-1%2Bdeb13u4_copyright'],
      ['changelog', 'https://metadata.ftp-master.debian.org/changelogs/main/n/nss/nss_3.110-1%2Bdeb13u4_changelog'],
      ['advisory-tracker', 'https://security-tracker.debian.org/tracker/source-package/nss'],
      ['source-descriptor', 'https://deb.debian.org/debian/pool/main/n/nss/nss_3.110-1+deb13u4.dsc'],
    ]) save(`${directory}/${name}.txt`, await download(url, 256 * 1024, deadline));
    for (const expected of packages) {
      stage = `download-${expected.name}`;
      const url = `https://deb.debian.org/debian/pool/main/n/nss/${expected.name}_3.110-1+deb13u4_amd64.deb`;
      const bytes = await download(url, 2 * MIB, deadline); requirePackageDigest(bytes, expected);
      save(`${directory}/${expected.name}.deb`, bytes);
      stage = `archive-${expected.name}`;
      const compressed = debianDataMember(bytes), compressedPath = `${directory}/${expected.name}.data.tar.xz`; save(compressedPath, compressed);
      const tar = await command(xz, ['--decompress', '--stdout', '--memlimit-decompress=256MiB', compressedPath]);
      const members = admitToolTar(tar); save(`${directory}/${expected.name}.data.tar`, tar);
      let selected = 0;
      for (const [name, member] of members) {
        const output = name === 'usr/bin/certutil' ? `${directory}/bin/certutil` : /^usr\/lib\/x86_64-linux-gnu\/lib(?:nss3|nssutil3|smime3|softokn3|freebl3|freeblpriv3|nssckbi|ssl3|nssdbm3)\.so$/.test(name) ? `${directory}/lib/${name.split('/').at(-1)}` : null;
        if (!output) continue;
        if (member.type !== '0') throw Error('tool selected file must be regular');
        save(output, member.bytes, name === 'usr/bin/certutil'); selected++;
      }
      record.packages.push({ ...expected, url, selected_regular_files: selected, admitted_members: members.size });
    }
    stage = 'dependency-probe';
    const libraryPath = `${directory}/lib:${nspr}:${sqlite}:${glibc}`;
    const resolved = (await command(loader, ['--library-path', libraryPath, '--list', `${directory}/bin/certutil`], 64 * 1024)).toString('utf8');
    const dependencies = [];
    for (const line of resolved.trim().split('\n')) {
      if (line.trim().startsWith('linux-vdso.so.1 ')) continue;
      const path = /(?:=>\s+)?(\/[^\s]+)\s+\(/.exec(line)?.[1];
      if (!path || ![`${directory}/lib/`, `${nspr}/`, `${sqlite}/`, `${glibc}/`].some(prefix => path.startsWith(prefix))) throw Error('unadmitted tool dependency');
      dependencies.push({ path, realpath: realpathSync(path), sha256: hash(readFileSync(path)) });
    }
    record.tool = { path: `${directory}/bin/certutil`, sha256: hash(readFileSync(`${directory}/bin/certutil`)), loader: { path: loader, realpath: realpathSync(loader), sha256: hash(readFileSync(loader)) }, library_path: libraryPath, dependencies };
    record.status = 'acquired-dependencies-admitted';
  } catch { record.status = 'tool-prerequisite-failed'; record.failure_stage = stage; process.exitCode = 1; }
  finally {
    record.drain = await launcher.drain(); record.group = snapshotProviderCgroup(group); requireDrainedCgroup(record.group.cgroup_events);
    record.stored_bytes = storedBytes(directory); record.total_retained_tool_bytes = totalToolBytes(); record.finished = new Date().toISOString();
    writeFileSync(`${evidence}/result.json`, JSON.stringify(record, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
    console.log(JSON.stringify({ status: record.status, evidence, tool_directory: directory, stored_bytes: record.stored_bytes, tls_admission: false }));
  }
}
