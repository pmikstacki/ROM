import { readFileSync, writeFileSync, mkdirSync, statfsSync, lstatSync,readlinkSync,realpathSync } from 'node:fs';
import { createHash, randomBytes, X509Certificate } from 'node:crypto';
import { createServer } from 'node:net';
import { fileURLToPath } from 'node:url';
import { OwnedLauncher } from './launch.mjs';
import { createProviderCgroup, admitOwnedWrapper, snapshotProviderCgroup, requireDrainedCgroup } from './cgroup.mjs';
import { admitBrowserStorage } from './browser-storage.mjs';
import {namespaceTools} from './trust-namespace.mjs';
const root = '/var/tmp/rom-010-authentik-20261007/run/volume';
const priorPath = `${root}/evidence/nss-private-trust-8b864cae6c63b27375327271/result.json`;
const acquiredPath = `${root}/evidence/nss-tool-ee61adb2a4df6c0baecd5f6e/result.json`;
const openssl = '/nix/store/48p8b28v6a8vma8dz19gf75f0jsb073w-openssl-3.3.3-bin/bin/openssl';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const fixtureModules=['tls-trust-probe.mjs','tls-browser-worker.mjs','trust-namespace.mjs','trust-browser-run.mjs','mount-ack.mjs','mount-command-run.mjs','crypto-wrapper.mjs','tls-observation.mjs','browser-storage.mjs','browser-paths.mjs','cgroup.mjs','launch.mjs','child-runner.mjs','supervision.mjs'];
const fixtureSources=()=>fixtureModules.map(name=>{const path=fileURLToPath(new URL(name,import.meta.url));return{path,sha256:hash(readFileSync(path))};});
async function vacant() {
  const server = createServer();
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(44389, '127.0.0.1', resolve); });
  await new Promise(resolve => server.close(resolve));
}
export async function probeBrowserTrust({cryptoWrapper=null}={}) {
  const hostTrust=()=>({mount_namespace:readlinkSync('/proc/self/ns/mnt'),ca_realpath:realpathSync('/etc/ssl/certs/ca-certificates.crt'),ca_sha256:hash(readFileSync('/etc/ssl/certs/ca-certificates.crt'))});
  const hostBefore=hostTrust();
  const gioPath = `${root}/evidence/gio-verified-ca8ffbb6ca5c63505ee4ac82/result.json`;
  const gio = JSON.parse(readFileSync(gioPath, 'utf8'));
  if (gio.status !== 'signed-historical-gio-verified' || hash(readFileSync(gio.module.path)) !== gio.module.sha256) throw Error('GIO module prerequisite');
  for (const dependency of gio.dependencies) if (hash(readFileSync(dependency.path)) !== dependency.sha256) throw Error('GIO module dependency drift');
  const prior = JSON.parse(readFileSync(priorPath, 'utf8')), acquired = JSON.parse(readFileSync(acquiredPath, 'utf8'));
  if (prior.status !== 'private-ca-import-readback-green' || acquired.status !== 'acquired-dependencies-admitted' || hash(readFileSync(openssl)) !== prior.openssl.sha256 || hash(new X509Certificate(readFileSync(prior.ca.certificate)).raw) !== prior.ca.der_sha256) throw Error('private TLS prerequisite changed');
  for (const file of prior.revalidated_files) if (hash(readFileSync(file.path)) !== file.sha256) throw Error('NSS tool changed');
  for (const file of [acquired.tool.loader, ...acquired.tool.dependencies]) if (hash(readFileSync(file.path)) !== file.sha256) throw Error('NSS dependency changed');
  const fs = statfsSync(root); if (Number(fs.bavail) * Number(fs.bsize) < 1024 ** 3) throw Error('TLS probe storage headroom');
  await vacant();
  const nonce = randomBytes(12).toString('hex'), privateRoot = `${root}/private/tls-${nonce}`, evidence = `${root}/evidence/tls-trust-${nonce}`;
  mkdirSync(privateRoot, { mode: 0o700 }); mkdirSync(evidence, { mode: 0o700 });
  const record = { schema: 'rom-isolated-browser-tls-trust-v1', status: 'starting', prior_prerequisite: priorPath, source_sha256: hash(readFileSync(fileURLToPath(import.meta.url))), cases: [], provider_acceptance: false, artifact_admission: false, port: '127.0.0.1:44389', gio_prerequisite: gioPath, runtime_supplement: gio.module, crypto_supplement:cryptoWrapper, production_dependency_admission: false };
  record.host_trust_before=hostBefore;
  record.fixture_source_before=fixtureSources();
  if(cryptoWrapper)record.namespace_tools=Object.fromEntries(Object.entries(namespaceTools).map(([name,path])=>[name,{path,realpath:realpathSync(path),sha256:hash(readFileSync(path))}]));
  const setupGroup = createProviderCgroup(), setupLauncher = new OwnedLauncher();
  let stage = 'synthetic-leaf';
  async function command(launcher, group, executable, args, env = { HOME: privateRoot, TMPDIR: privateRoot, PATH: '/run/current-system/sw/bin:/usr/bin:/bin', LANG: 'C', TZ: 'UTC' }) {
    const child = launcher.launch(executable, args, { timeoutMs: 30000, graceMs: 1000, drainMs: 1000, outputBytes: 1024 ** 2, env, admitIdentity: identity => admitOwnedWrapper(group, identity) });
    let stdoutBytes = 0, stderrBytes = 0; child.child.stdout.on('data', bytes => { stdoutBytes += bytes.length; }); child.child.stderr.on('data', bytes => { stderrBytes += bytes.length; });
    const target = await child.targetStarted, result = await child.closed; await child.physicalClose;
    const process = { wrapper: child.identity, target, result, executable, stdout_bytes: stdoutBytes, stderr_bytes: stderrBytes };
    if (!target || result.exit_code !== 0 || !result.drained || result.signal !== null) throw Error('TLS tool prerequisite failed');
    return process;
  }
  const leaves = {};
  try {
    record.setup_processes = [];
    for (const kind of ['trusted', 'wrong-san', 'expired']) {
      const directory = `${privateRoot}/${kind}`; mkdirSync(directory, { mode: 0o700 }); mkdirSync(`${directory}/issued`, { mode: 0o700 });
      const key = `${directory}/key.pem`, certificate = `${directory}/certificate.pem`, csr = `${directory}/request.pem`, config = `${directory}/openssl.cnf`;
      writeFileSync(`${directory}/index`, '', { flag: 'wx', mode: 0o600 }); writeFileSync(`${directory}/serial`, '01\n', { flag: 'wx', mode: 0o600 });
      writeFileSync(config, `[req]\nprompt=no\ndistinguished_name=subject\n[subject]\nCN=ROM synthetic TLS server\n[ca]\ndefault_ca=signing\n[signing]\ndatabase=${directory}/index\nserial=${directory}/serial\nnew_certs_dir=${directory}/issued\ncertificate=${prior.ca.certificate}\nprivate_key=${prior.home}/ca-key.pem\ndefault_md=sha256\ndefault_days=1\npolicy=policy\nx509_extensions=extensions\n[policy]\ncommonName=supplied\n[extensions]\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=${kind === 'wrong-san' ? 'DNS:wrong.invalid' : 'IP:127.0.0.1'}\n`, { flag: 'wx', mode: 0o600 });
      record.setup_processes.push(await command(setupLauncher, setupGroup, openssl, ['req', '-new', '-newkey', 'rsa:2048', '-nodes', '-config', config, '-keyout', key, '-out', csr]));
      const date = time => new Date(time).toISOString().replace(/[-:TZ.]/g, '').slice(2, 14) + 'Z';
      const args = ['ca', '-batch', '-notext', '-config', config, '-in', csr, '-out', certificate];
      if (kind === 'expired') args.push('-startdate', date(Date.now() - 172800000), '-enddate', date(Date.now() - 86400000));
      record.setup_processes.push(await command(setupLauncher, setupGroup, openssl, args));
      if (lstatSync(key).mode & 0o077) throw Error('TLS private key permissions');
      const cert = new X509Certificate(readFileSync(certificate));
      if (kind === 'wrong-san' ? cert.checkIP('127.0.0.1') !== undefined : cert.checkIP('127.0.0.1') !== '127.0.0.1') throw Error('synthetic SAN precondition');
      if (kind === 'expired' ? Date.parse(cert.validTo) >= Date.now() : Date.parse(cert.validTo) <= Date.now()) throw Error('synthetic expiry precondition');
      leaves[kind] = { key, certificate, der_sha256: hash(cert.raw), valid_from: cert.validFrom, valid_to: cert.validTo, subject_alt_name: cert.subjectAltName };
    }
    record.setup_drain = await setupLauncher.drain(); record.setup_group = snapshotProviderCgroup(setupGroup); requireDrainedCgroup(record.setup_group.cgroup_events);
    record.leaves = leaves;
    stage = 'browser-trust';
    for (const engine of ['chromium', 'webkit']) for (const expected of ['trusted', 'untrusted-ca', 'wrong-san', 'expired']) {
      await vacant();
      const directory = `${privateRoot}/${engine}-${expected}`, home = `${directory}/home`, temporary = `${root}/t-${randomBytes(4).toString('hex')}`;
      mkdirSync(directory, { mode: 0o700 }); mkdirSync(home, { mode: 0o700 }); mkdirSync(temporary, { mode: 0o700 }); mkdirSync(`${home}/.pki`, { mode: 0o700 }); mkdirSync(`${home}/.pki/nssdb`, { mode: 0o700 });
      const group = createProviderCgroup(), launcher = new OwnedLauncher();
      const item = { engine, expected, home, temporary, processes: [], storage: admitBrowserStorage(root, { home, temporary }) }; record.cases.push(item);
      try {
        const env = { HOME: home, TMPDIR: temporary, PATH: '/run/current-system/sw/bin:/usr/bin:/bin', LANG: 'C', TZ: 'UTC', SSL_CERT_FILE: prior.ca.certificate, NIX_SSL_CERT_FILE: prior.ca.certificate, GIO_EXTRA_MODULES: gio.module.env_module_directory, PLAYWRIGHT_BROWSERS_PATH: '/root/.cache/ms-playwright' };
        if (expected === 'untrusted-ca') { const empty = `${home}/empty-ca.pem`; writeFileSync(empty, '', { flag: 'wx', mode: 0o600 }); env.SSL_CERT_FILE = empty; env.NIX_SSL_CERT_FILE = empty; }
        else if (engine === 'chromium') {
          const nss = args => command(launcher, group, acquired.tool.loader.path, ['--library-path', acquired.tool.library_path, acquired.tool.path, ...args], env);
          item.processes.push(await nss(['-N', '--empty-password', '-d', `sql:${home}/.pki/nssdb`]));
          item.processes.push(await nss(['-A', '-d', `sql:${home}/.pki/nssdb`, '-n', 'ROM isolated identity fixture CA', '-t', 'C,,', '-i', prior.ca.certificate]));
        }
        const leaf = leaves[expected === 'untrusted-ca' ? 'trusted' : expected], resultPath = `${evidence}/${engine}-${expected}.json`, configuration = `${directory}/configuration.json`;
        const isolatedTrust=engine==='webkit'&&cryptoWrapper,trustView=`${directory}/trust-view`,namespaceResult=`${evidence}/${engine}-${expected}-namespace.json`;
        if(isolatedTrust){mkdirSync(trustView,{mode:0o700});writeFileSync(`${trustView}/ca-certificates.crt`,expected==='untrusted-ca'?'':readFileSync(prior.ca.certificate),{flag:'wx',mode:0o600});}
        writeFileSync(configuration, JSON.stringify({ engine, expected, key: leaf.key, certificate: leaf.certificate, result: resultPath,crypto_wrapper:engine==='webkit'?cryptoWrapper:null,...(isolatedTrust?{trust_view:trustView,parent_mount_namespace:hostBefore.mount_namespace,namespace_result:namespaceResult}:{}) }), { flag: 'wx', mode: 0o600 });
        const executable=isolatedTrust?namespaceTools.unshare:process.execPath,args=isolatedTrust?['--mount','--propagation','private','--fork',process.execPath,fileURLToPath(new URL('./trust-browser-run.mjs',import.meta.url)),configuration]:[fileURLToPath(new URL('./tls-browser-worker.mjs', import.meta.url)), configuration];
        const child = launcher.launch(executable, args, { timeoutMs: 90000, graceMs: 1000, drainMs: 1000, outputBytes: 1024 ** 2, env, admitIdentity: identity => admitOwnedWrapper(group, identity) });
        let stdoutBytes = 0, stderrBytes = 0; child.child.stdout.on('data', bytes => { stdoutBytes += bytes.length; }); child.child.stderr.on('data', bytes => { stderrBytes += bytes.length; });
        const target = await child.targetStarted, result = await child.closed; await child.physicalClose;
        item.processes.push({ wrapper: child.identity, target, result, stdout_bytes: stdoutBytes, stderr_bytes: stderrBytes });
        if(isolatedTrust)item.namespace=JSON.parse(readFileSync(namespaceResult,'utf8'));
        item.observation = JSON.parse(readFileSync(resultPath, 'utf8')); item.status = target && result.exit_code === 0 && result.drained && item.observation.status === 'passed' ? 'passed' : 'prerequisite-failed';
      } catch { item.status = 'prerequisite-failed'; }
      finally { item.drain = await launcher.drain(); item.group = snapshotProviderCgroup(group); requireDrainedCgroup(item.group.cgroup_events); await vacant(); }
    }
    record.status = record.cases.length === 8 && record.cases.every(item => item.status === 'passed') ? 'browser-tls-trust-prerequisite-green' : 'browser-tls-trust-prerequisite-failed';
  } catch { record.status = 'browser-tls-trust-prerequisite-failed'; record.failure_stage = stage; }
  finally {
    await setupLauncher.drain(); record.final_setup_group = snapshotProviderCgroup(setupGroup); requireDrainedCgroup(record.final_setup_group.cgroup_events);
    if (record.status !== 'browser-tls-trust-prerequisite-green') process.exitCode = 1;
    record.host_trust_after=hostTrust();record.host_trust_unchanged=JSON.stringify(hostBefore)===JSON.stringify(record.host_trust_after);if(!record.host_trust_unchanged){record.status='browser-tls-trust-prerequisite-failed';process.exitCode=1;}
    record.fixture_source_after=fixtureSources();record.fixture_source_unchanged=JSON.stringify(record.fixture_source_before)===JSON.stringify(record.fixture_source_after);if(!record.fixture_source_unchanged){record.status='browser-tls-trust-prerequisite-failed';process.exitCode=1;}
    writeFileSync(`${evidence}/result.json`, JSON.stringify(record, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
    console.log(JSON.stringify({ status: record.status, evidence, provider_acceptance: false, artifact_admission: false }));
  }
}
