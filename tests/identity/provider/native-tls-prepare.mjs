// Preparation only. This module does not launch native code, listeners or browsers.
import { createHash, createPrivateKey, X509Certificate, randomBytes } from 'node:crypto';
import { lstatSync, realpathSync, mkdirSync, writeFileSync, statfsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { inspectNativeTlsBrowserGraph } from './native-tls-browser-graph.mjs';
import { nativeTlsEnvironment, selectNativeTlsCase, NATIVE_TLS_PROVIDER_REMAINING_MS } from './native-tls-profile.mjs';
import { readNativeTlsFile } from './native-tls-io.mjs';
import { requireBrowserSocketBudget } from './browser-paths.mjs';
import { admitBrowserStorage } from './browser-storage.mjs';
const volume = '/var/tmp/rom-010-authentik-20261007/run/volume';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');

function file(path, maximum, privateFile = false) {
  const bytes = readNativeTlsFile(path, maximum, privateFile);
  return { path, sha256: digest(bytes), bytes };
}
function witness(record) { const { bytes, ...result } = record; return result; }

export function requireNativeTlsBrowserTemporary(expected, root = volume) {
  requireBrowserSocketBudget(expected?.path);
  if (Object.keys(expected).sort().join(',') !== 'birth_ms,device,inode,mode,path,uid' || !expected.path.startsWith(root + '/t-') || !/^[a-f0-9]{8}$/.test(expected.path.slice((root + '/t-').length))) throw Error('browser temporary identity changed');
  const actual = lstatSync(expected.path), parent = lstatSync(root);
  if (realpathSync(root) !== root || realpathSync(expected.path) !== expected.path || !actual.isDirectory() || actual.isSymbolicLink() || actual.dev !== parent.dev || actual.uid !== process.getuid() || (actual.mode & 0o777) !== 0o700 || actual.dev !== expected.device || actual.ino !== expected.inode || actual.birthtimeMs !== expected.birth_ms || actual.uid !== expected.uid || (actual.mode & 0o777) !== expected.mode) throw Error('browser temporary identity changed');
  admitBrowserStorage(root, { home: expected.path, temporary: expected.path });
  return expected;
}

export function createNativeTlsBrowserTemporary(root = volume) {
  for (let attempt = 0; attempt < 16; attempt++) {
    const path = root + '/t-' + randomBytes(4).toString('hex'); requireBrowserSocketBudget(path);
    try { mkdirSync(path, { mode: 0o700 }); } catch (error) { if (error.code === 'EEXIST') continue; throw error; }
    const value = lstatSync(path), expected = { path, device: value.dev, inode: value.ino, birth_ms: value.birthtimeMs, uid: value.uid, mode: value.mode & 0o777 };
    return requireNativeTlsBrowserTemporary(expected, root);
  }
  throw Error('fresh browser temporary directory collision bound');
}

export function validateNativeTlsOptions(options) {
  const names = ['adapter', 'binary_identity', 'case_certificate', 'case_route', 'client_input', 'native_ca', 'provider_ca', 'environment', 'nss_reference', 'provider_ready', 'subject_input', 'trusted_leaf', 'negative_leaf', 'negative_ca'];
  if (!options || Object.keys(options).sort().join(',') !== names.sort().join(',')) throw Error('closed native TLS preparation options required');
  return selectNativeTlsCase({ adapter: options.adapter, route: options.case_route, certificate: options.case_certificate });
}

export function validateNativeTlsMaterial(selected, { nativeCa, providerCa, negativeCa = providerCa, trusted, negative }) {
  const current = certificate => Date.parse(certificate.validFrom) <= Date.now() && Date.parse(certificate.validTo) > Date.now();
  for (const root of [nativeCa, providerCa, negativeCa]) if (!root.ca || !current(root)) throw Error('current CA certificates required');
  if (trusted.ca || !trusted.verify(providerCa.publicKey) || !current(trusted) || trusted.checkIP('127.0.0.1') !== '127.0.0.1') throw Error('trusted provider leaf must be current, match loopback and have the provider signature');
  if (negative.ca || !negative.verify(negativeCa.publicKey)) throw Error('negative leaf must have its declared CA signature');
  if (selected.certificate === 'wrong-ca' && selected.route === 'token') {
    if (trusted.verify(nativeCa.publicKey) || !trusted.raw.equals(negative.raw) || !negativeCa.raw.equals(providerCa.raw)) throw Error('token wrong root requires unrelated native CA and unchanged provider leaf');
    return;
  }
  if (!nativeCa.raw.equals(providerCa.raw)) throw Error('token acquisition must initially trust the provider CA');
  if (selected.certificate === 'wrong-ca') {
    if (negativeCa.raw.equals(providerCa.raw) || negative.verify(providerCa.publicKey) || !current(negative) || negative.checkIP('127.0.0.1') !== '127.0.0.1') throw Error('JWKS wrong issuer requires a current loopback leaf signed by an unrelated CA');
    return;
  }
  if (!negativeCa.raw.equals(providerCa.raw)) throw Error('positive, name and expiry controls retain the provider CA');
  if (selected.certificate === 'trusted' && !trusted.raw.equals(negative.raw)) throw Error('positive control leaf must remain unchanged');
  if (selected.certificate === 'wrong-name' && (!current(negative) || negative.checkIP('127.0.0.1') === '127.0.0.1')) throw Error('wrong-name leaf must be current with a mismatched SAN');
  if (selected.certificate === 'expired' && (Date.parse(negative.validTo) > Date.now() || negative.checkIP('127.0.0.1') !== '127.0.0.1')) throw Error('expired leaf must match name and have an expired validity interval');
}

export function prepareNativeTls(options) {
  const selected = validateNativeTlsOptions(options);
  for (const path of [options.binary_identity, options.provider_ready, options.client_input, options.subject_input, options.native_ca, options.provider_ca, options.negative_ca, options.environment, options.nss_reference]) if (!path.startsWith(`${volume}/`)) throw Error('owned identity input required');
  const buildRecord = file(options.binary_identity, 1048576), build = JSON.parse(buildRecord.bytes);
  if (!Array.isArray(build.sources) || build.sources.length < 1 || build.sources.length > 4096) throw Error('complete native source witness required');
  if (new Set(build.sources.map(source => source.path)).size !== build.sources.length) throw Error('duplicate native source witness');
  const sources = build.sources.map(source => {
    const value = file(source.path, 16 * 1024 ** 2);
    if (value.sha256 !== source.sha256) throw Error('native source fence changed');
    return witness(value);
  });
  for (const suffix of ['host/src/provisioning.rs', 'host/src/host.rs']) if (!sources.some(source => source.path.endsWith(suffix))) throw Error('native TLS host source witness missing');
  const binary = file(build.binary?.path, 128 * 1024 ** 2);
  if (binary.sha256 !== build.binary.sha256) throw Error('native binary identity changed');
  const readyRecord = file(options.provider_ready, 1048576), ready = JSON.parse(readyRecord.bytes);
  if (!ready.relay?.target || !Number.isSafeInteger(ready.relay.target.pid)) throw Error('owned provider relay identity required');
  const clientRecord = file(options.client_input, 65536, true), client = JSON.parse(clientRecord.bytes);
  const subjectRecord = file(options.subject_input, 65536, true), subject = JSON.parse(subjectRecord.bytes);
  if (typeof client.client_id !== 'string' || typeof client.client_secret !== 'string' || !client.client_secret || typeof subject.subject !== 'string' || !subject.subject) throw Error('original provider client and verified subject inputs required');
  const providerCa = file(options.provider_ca, 65536), environment = file(options.environment, 65536, true), nssRecord = file(options.nss_reference, 1048576);
  if ((lstatSync(environment.path).mode & 0o077) !== 0) throw Error('private provider environment permissions required');
  const ca = file(options.native_ca, 65536), certificate = new X509Certificate(ca.bytes);
  if (!certificate.ca || Date.parse(certificate.validFrom) > Date.now() || Date.parse(certificate.validTo) <= Date.now()) throw Error('current native CA required');
  const negativeCaRecord = file(options.negative_ca, 65536);
  const negativeCa = new X509Certificate(negativeCaRecord.bytes);
  const browserCa = new X509Certificate(providerCa.bytes);
  if (!browserCa.ca || Date.parse(browserCa.validFrom) > Date.now() || Date.parse(browserCa.validTo) <= Date.now()) throw Error('current provider CA required');
  const leafInputs = [], parsedLeaves = [];
  for (const leaf of [options.trusted_leaf, options.negative_leaf]) {
    if (!leaf || Object.keys(leaf).sort().join(',') !== 'certificate,key') throw Error('closed leaf references required');
    for (const path of [leaf.certificate, leaf.key]) if (!path.startsWith(`${volume}/`)) throw Error('owned private TLS leaf required');
    const cert = file(leaf.certificate, 65536), key = file(leaf.key, 65536, true);
    if ((lstatSync(key.path).mode & 0o077) !== 0) throw Error('private leaf key permissions required');
    const parsed = new X509Certificate(cert.bytes);
    if (parsed.ca || !parsed.checkPrivateKey(createPrivateKey(key.bytes))) throw Error('owned provider leaf, issuer signature and matching private key required');
    parsedLeaves.push(parsed);
    leafInputs.push({ certificate: witness(cert), key: witness(key) });
  }
  validateNativeTlsMaterial(selected, { nativeCa: certificate, providerCa: browserCa, negativeCa, trusted: parsedLeaves[0], negative: parsedLeaves[1] });
  const free = statfsSync(`${volume}/private`, { bigint: true });
  if (free.bavail * free.bsize < 2n * 1024n ** 3n) throw Error('native fixture admission requires 2 GiB free');
  const directory = `${volume}/private/native-tls-${randomBytes(12).toString('hex')}`;
  mkdirSync(directory, { mode: 0o700 });
  for (const name of ['empty-ca', 'home', 'tmp', 'handoff', 'controls']) mkdirSync(`${directory}/${name}`, { mode: 0o700 });
  const browserTemporary = createNativeTlsBrowserTemporary();
  const configuration = { adapter: selected.adapter, assets_directory: '/root/ROM/tests/identity/provider/host/assets', database: `${directory}/database`, stop_file: `${directory}/host.stop`, control_directory: `${directory}/controls`, public_origin: 'https://127.0.0.1:44389', private_token_endpoint: null, private_jwks_endpoint: null, issuer: 'https://127.0.0.1:44392/application/o/rom-synthetic-identity/', client_id: client.client_id, client_secret: client.client_secret, authorization_endpoint: 'https://127.0.0.1:44392/application/o/authorize/', token_endpoint: 'https://127.0.0.1:44392/application/o/token/', jwks_endpoint: 'https://127.0.0.1:44392/application/o/rom-synthetic-identity/jwks/', verified_synthetic_subject: subject.subject };
  const configurationPath = `${directory}/host.json`;
  writeFileSync(configurationPath, JSON.stringify(configuration), { flag: 'wx', mode: 0o600 });
  const fixtureSources = ['native-tls-prepare.mjs', 'native-tls-prepare-run.mjs', 'native-tls-browser-graph.mjs', 'browser-paths.mjs', 'browser-storage.mjs', 'native-tls-profile.mjs', 'native-tls-handoff.mjs', 'native-tls-callback-hold.mjs', 'native-tls-io.mjs', 'native-tls-storage.mjs', 'native-tls-proxy.mjs', 'native-tls-proxy-run.mjs', 'native-tls-browser.mjs', 'native-tls-browser-observation.mjs', 'native-tls-browser-run.mjs', 'native-tls-orchestrator.mjs', 'native-tls-stage-observer.mjs', 'native-tls-run.mjs', 'launch.mjs', 'child-runner.mjs', 'supervision.mjs', 'cgroup.mjs', 'network-admission.mjs', 'proxy-contract.mjs', 'stop-file.mjs'].map(name => witness(file(fileURLToPath(new URL(name, import.meta.url)), 1048576)));
  fixtureSources.push(witness(file('/root/ROM/studio/package-lock.json', 16 * 1024 ** 2)));
  const browserGraph = inspectNativeTlsBrowserGraph('/root/ROM/studio');
  const browserInputs = [...browserGraph.files];
  browserInputs.push(witness(file(realpathSync('/root/.nix-profile/bin/chromium'), 16 * 1024 ** 2)));
  const nss = JSON.parse(nssRecord.bytes);
  if (!nss.tool?.loader || !Array.isArray(nss.tool.dependencies) || nss.tool.dependencies.length > 64) throw Error('admitted private NSS dependency graph required');
  for (const value of [nss.tool, nss.tool.loader, ...nss.tool.dependencies]) {
    const observed = file(value.path, 128 * 1024 ** 2);
    if (observed.sha256 !== value.sha256) throw Error('private NSS input changed');
    browserInputs.push(witness(observed));
  }
  const record = {
    schema: 'rom-native-tls-preparation-v1', status: 'prepared-not-executed', directory, selected_case: selected,
    inputs: [witness(buildRecord), witness(readyRecord), witness(clientRecord), witness(subjectRecord), witness(ca), witness(providerCa), witness(negativeCaRecord), witness(environment), witness(nssRecord)],
    provider_ready: options.provider_ready, provider_ca: options.provider_ca, environment: options.environment, nss_reference: options.nss_reference, binary: witness(binary), native_sources: sources, fixture_sources: fixtureSources, browser_inputs: browserInputs, browser_graph: browserGraph, browser_temporary: browserTemporary, leaves: leafInputs,
    host_configuration: witness(file(configurationPath, 65536)),
    host_command: { executable: binary.path, args: [configurationPath], env: nativeTlsEnvironment({ ca: ca.path, emptyCaDirectory: `${directory}/empty-ca`, home: `${directory}/home`, temporary: `${directory}/tmp` }) },
    budgets: { outer_case_ms: 210000, provider_remaining_ms: NATIVE_TLS_PROVIDER_REMAINING_MS, host_timeout_ms: 130000, proxy_timeout_ms: 130000, browser_timeout_ms: 60000, acquisition_timeout_ms: 3000, wrapper_admission_timeout_ms: 1000, grace_ms: 1000, drain_ms: 1000, output_bytes_per_process: 1048576, proxy_requests: 128, proxy_response_bytes: 33554432, planned_allocated_bytes: 1073741824, aggregate_hard_quota: false },
    required_launch_checks: ['source and binary fences', 'provider relay birth identity and owned listener 44390', 'vacant 44389/44391/44392', 'fresh private browser trust', 'owned cgroup admission before target starts', 'target executable and child trust environment witness', 'browser drain before native callback', 'provider TLS connection close after original token before JWKS'],
    required_completion_checks: ['actual native token/JWKS route oracle', 'owned stop then launcher drain and physical close', 'drained cgroups and vacant allocated listeners', 'source/binary/CA fences after run', 'allocated private file inventory against planned budget'],
    unresolved_execution: ['finite owned provider/proxy/browser lease and independent source review required'],
    production_dependency_admission: false, artifact_admission: false, original_consumer_acceptance: false,
  };
  writeFileSync(`${directory}/preparation.json`, JSON.stringify(record, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
  return record;
}
