// Execute the native fixture test binary on the admitted hard run filesystem.
import { readFileSync, writeFileSync, mkdirSync, realpathSync, statfsSync } from 'node:fs';
import { createHash, randomBytes } from 'node:crypto';
import { OwnedLauncher } from './launch.mjs';
import { createProviderCgroup, admitOwnedWrapper, snapshotProviderCgroup, requireDrainedCgroup } from './cgroup.mjs';
const root = '/var/tmp/rom-010-authentik-20261007/run/volume';
const input = `${root}/evidence/lifecycle-20261007-red/compile-https-green.jsonl`;
const records = readFileSync(input, 'utf8').trim().split('\n').filter(line => line.startsWith('{')).map(line => JSON.parse(line));
const build = records.find(record => record.reason === 'compiler-artifact' && record.target.name === 'rom-identity-host-authoring' && record.profile.test && record.executable);
if (!records.some(record => record.reason === 'build-finished' && record.success) || !build || !build.executable.startsWith('/workspace/ROM/target/debug/deps/')) throw Error('fixture test executable missing');
const executable = build.executable.replace('/workspace/ROM/', '/root/ROM/');
if (realpathSync(executable) !== executable || Number(statfsSync(root).bavail) * Number(statfsSync(root).bsize) < 1024 ** 3) throw Error('fixture storage prerequisite');
const evidence = `${root}/evidence/lifecycle-unit-${randomBytes(12).toString('hex')}`;
mkdirSync(evidence, { mode: 0o700 });
const group = createProviderCgroup(), launcher = new OwnedLauncher();
const record = { schema: 'rom-identity-fixture-unit-authoring-v1', executable, sha256: createHash('sha256').update(readFileSync(executable)).digest('hex'), run_root: root, status: 'starting', provider_acceptance: false };
try {
  const child = launcher.launch(executable, ['control_tests', '--test-threads=1'], { timeoutMs: 60000, graceMs: 1000, drainMs: 1000, outputBytes: 1024 ** 2, admitIdentity: identity => admitOwnedWrapper(group, identity) });
  const output = [];
  child.child.stdout.on('data', bytes => output.push(bytes)); child.child.stderr.on('data', bytes => output.push(bytes));
  record.wrapper = child.identity; record.target = await child.targetStarted; record.result = await child.closed; await child.physicalClose;
  writeFileSync(`${evidence}/test.log`, Buffer.concat(output), { flag: 'wx', mode: 0o600 });
  record.status = record.target && record.result.exit_code === 0 && record.result.drained ? 'fixture-unit-green' : 'fixture-unit-failed';
  if (record.status !== 'fixture-unit-green') process.exitCode = 1;
} finally {
  record.drain = await launcher.drain(); record.group = snapshotProviderCgroup(group); requireDrainedCgroup(record.group.cgroup_events);
  writeFileSync(`${evidence}/result.json`, JSON.stringify(record, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
  console.log(JSON.stringify({ status: record.status, evidence, provider_acceptance: false }));
}
