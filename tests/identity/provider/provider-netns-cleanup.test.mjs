import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';

const unique = 'a'.repeat(32);
const path = '/var/tmp/rom-010-authentik-20261007/run/volume/private/authentik-96b706d518977e471f458dfff7b9503a/provider-netns-' + unique;
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;

// Execute the maintained finalizer; only provider/kernel boundaries are substituted.
async function finalize(cleanup) {
  const source = fs.readFileSync(new URL('./lifecycle-provider-run.mjs', import.meta.url), 'utf8');
  const marker = '\nfinally{';
  const body = source.slice(source.lastIndexOf(marker) + marker.length, source.lastIndexOf('}'));
  const record = {unique, status: 'ready', containers: [], namespace_mount: {file: path}, fixture_source_before: []};
  const process = {exitCode: 0};
  const events = [];
  const values = {
    clearInterval() {}, monitor: null, record, process, group: {},
    relayLauncher: {async shutdown() { events.push('relay'); return []; }},
    provider: {async stop() {}, async drain() { events.push('provider'); return []; }, records() { return {}; }},
    mailbox: {records() { return []; }},
    readFileSync() { throw Error('unexpected file read'); },
    createHash() { throw Error('unexpected source hash'); },
    async cleanupProviderNetns() { events.push('namespace'); return cleanup; },
    async persist() { events.push('terminal'); },
    console: {log() {}}, unique,
  };
  await new AsyncFunction(...Object.keys(values), body)(...Object.values(values));
  return {record, process, events};
}

test('successful provider finalization records namespace cleanup before its terminal', async () => {
  const result = await finalize({status: 'unmounted', passed: true, path});
  assert.deepEqual(result.events, ['relay', 'provider', 'namespace', 'terminal']);
  assert.equal(result.record.namespace_unmount?.passed, true);
  assert.equal(result.process.exitCode, 0);
});

test('namespace cleanup failure cannot leave a ready terminal or successful process exit', async () => {
  const result = await finalize({status: 'failed', passed: false, path});
  assert.equal(result.record.namespace_unmount?.passed, false);
  assert.equal(result.record.status, 'cleanup-failed');
  assert.equal(result.process.exitCode, 1);
  assert.equal(result.events.at(-1), 'terminal');
});

const identity = {pid: 123, group: 123, session: 123, started: '456', boot_id: 'synthetic-boot'};
const group = {path: '/sys/fs/cgroup/rom-identity-' + 'b'.repeat(32), inode: 42};
const original = {device: 10, inode: 20, uid: 0, mode: 0o600, nlink: 1, bytes: 0};
const allocation = {
  file: path, namespace: 'net:[777]', mount_namespace: 'mnt:[888]', backing_file: original,
  directory: {path: path.slice(0, path.lastIndexOf('/')), device: 10, inode: 19, uid: 0, mode: 0o700},
};
const witness = {mount_id: '52', parent_id: '42', device: '0:4', root: 'net:[777]', target: path,
  filesystem: 'nsfs', source: 'nsfs', file: {device: 4, inode: 777, uid: 0, mode: 0o444, nlink: 1, bytes: 0}};
const mountLine = `52 42 0:4 net:[777] ${path} rw - nsfs nsfs rw\n`;

function fixture({mounted = true} = {}) {
  const state = {mounted, line: mountLine, live: false, empty: true, calls: [], keepMounted: false,
    closed: {exit_code: 0, signal: null, reason: 'completed', drained: true}, physical: {code: 0, signal: null}};
  const stat = value => ({...value, dev: value.device, ino: value.inode, size: value.bytes,
    isFile: () => true, isDirectory: () => false, isSymbolicLink: () => false});
  const io = {
    realpathSync: value => value,
    readlinkSync: () => 'mnt:[888]',
    lstatSync(value) {
      if (value === allocation.directory.path) return {...stat(allocation.directory), isFile: () => false, isDirectory: () => true};
      assert.equal(value, path);
      return stat(state.mounted ? witness.file : original);
    },
  };
  const hooks = {io, uid: 0, readMountInfo: () => state.mounted ? state.line : '',
    readIdentity: () => state.live ? identity : null,
    snapshotGroup: () => ({...group, cgroup_events: `populated ${state.empty ? 0 : 1}\nfrozen 0\n`}),
    async unmount(value, selectedGroup, admit) {
      state.beforeAdmission?.();
      admit?.();
      state.calls.push(value);
      assert.equal(value, path);
      assert.equal(selectedGroup, group);
      if (!state.keepMounted && state.physical.code === 0 && state.closed.exit_code === 0) state.mounted = false;
      return {wrapper: identity, target: {...identity, pid: 124}, closed: state.closed, physical: state.physical,
        drain: [{...state.closed}], group: {...group, cgroup_events: 'populated 0\nfrozen 0\n'}};
    },
  };
  const record = {unique, namespace_mount: {...structuredClone(allocation), witness: structuredClone(witness)},
    containers: [{observation: {birth: identity}}], relay_processes: [], stop_results: [{stopped: true}], provider_drain: [], records: {}};
  return {state, hooks, record};
}

const module = () => import('./provider-netns-cleanup.mjs');

test('the exact retained nsfs mount is removed after owners drain', async () => {
  const {cleanupProviderNetns} = await module();
  const f = fixture();
  const result = await cleanupProviderNetns(f.record, group, f.hooks);
  assert.equal(result.passed, true);
  assert.equal(result.status, 'unmounted');
  assert.equal(f.state.mounted, false);
  assert.deepEqual(f.state.calls, [path]);
  assert.equal(result.physical.code, 0);
});

test('an allocated file with no mount is retained without an unmount command', async () => {
  const {cleanupProviderNetns} = await module();
  const f = fixture({mounted: false});
  delete f.record.namespace_mount.witness;
  const result = await cleanupProviderNetns(f.record, group, f.hooks);
  assert.equal(result.passed, true);
  assert.equal(result.status, 'not-mounted');
  assert.deepEqual(f.state.calls, []);
});

test('setup failure before allocation needs no namespace command', async () => {
  const {cleanupProviderNetns} = await module();
  const f = fixture({mounted: false});
  delete f.record.namespace_mount;
  assert.equal((await cleanupProviderNetns(f.record, group, f.hooks)).status, 'not-created');
  assert.deepEqual(f.state.calls, []);
});

for (const [name, change] of [
  ['another unique path', f => f.record.namespace_mount.file = path.replace(unique, 'c'.repeat(32))],
  ['different network namespace', f => f.record.namespace_mount.namespace = 'net:[778]'],
  ['replaced mount identity', f => f.state.line = mountLine.replace('52 42', '53 42')],
  ['non-nsfs replacement', f => f.state.line = mountLine.replace('nsfs nsfs', 'tmpfs tmpfs')],
  ['stacked mounts', f => f.state.line += mountLine],
  ['missing captured mount witness', f => delete f.record.namespace_mount.witness],
  ['live owned init', f => f.state.live = true],
  ['live restarted owner', f => {f.record.containers = []; f.record.records = {containers: [{pid: identity}]}; f.state.live = true;}],
  ['nonempty provider group', f => f.state.empty = false],
  ['failed container stop', f => f.record.stop_results[0].stopped = false],
  ['undrained relay', f => f.record.relay_processes = [{drained: false}]],
  ['undrained engine', f => f.record.provider_drain = [{drained: false}]],
]) test('refuses unsafe unmount for ' + name, async () => {
  const {cleanupProviderNetns} = await module();
  const f = fixture(); change(f);
  const result = await cleanupProviderNetns(f.record, group, f.hooks);
  assert.equal(result.passed, false);
  assert.deepEqual(f.state.calls, []);
  assert.equal(f.state.mounted, true);
});

for (const [name, change] of [
  ['nonzero unmount', f => f.state.closed.exit_code = 1],
  ['nonzero physical exit', f => f.state.physical.code = 1],
  ['undrained command', f => f.state.closed.drained = false],
  ['deadline termination', f => f.state.closed.reason = 'deadline'],
  ['mount still present', f => f.state.keepMounted = true],
]) test('does not report cleanup success after ' + name, async () => {
  const {cleanupProviderNetns} = await module();
  const f = fixture(); change(f);
  const result = await cleanupProviderNetns(f.record, group, f.hooks);
  assert.equal(result.passed, false);
  assert.deepEqual(f.state.calls, [path]);
});

test('allocation and mount capture bind inode and current mount namespace', async () => {
  const {createProviderNetnsAllocation, captureProviderNetnsMount} = await module();
  const f = fixture({mounted: false});
  const allocated = createProviderNetnsAllocation(unique, 'net:[777]', f.hooks);
  assert.deepEqual(allocated, allocation);
  f.state.mounted = true;
  assert.deepEqual(captureProviderNetnsMount(allocated, f.hooks), witness);
});

test('a failed mount command that left the captured owned mount is still cleaned', async () => {
  const {cleanupProviderNetns} = await module();
  const f = fixture();
  f.record.namespace_mount.result = {exit_code: 1, drained: true};
  const result = await cleanupProviderNetns(f.record, group, f.hooks);
  assert.equal(result.passed, true);
  assert.equal(f.state.mounted, false);
});

test('mount replacement before target admission does not execute unmount', async () => {
  const {cleanupProviderNetns} = await module();
  const f = fixture();
  f.state.beforeAdmission = () => { f.state.line = mountLine.replace('52 42', '53 42'); };
  const result = await cleanupProviderNetns(f.record, group, f.hooks);
  assert.equal(result.passed, false);
  assert.deepEqual(f.state.calls, []);
  assert.equal(f.state.mounted, true);
});
