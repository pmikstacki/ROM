import fs from 'node:fs';
import {isDeepStrictEqual} from 'node:util';
import {OwnedLauncher, readProcessIdentity, sameProcessIdentity} from './launch.mjs';
import {admitOwnedWrapper, snapshotProviderCgroup, requireDrainedCgroup} from './cgroup.mjs';

const directory = '/var/tmp/rom-010-authentik-20261007/run/volume/private/authentik-96b706d518977e471f458dfff7b9503a';
const mountInfoBound = 1024 * 1024;
const fileIdentity = value => ({device: value.dev, inode: value.ino, uid: value.uid,
  mode: value.mode & 0o777, nlink: value.nlink, bytes: value.size});

function readMountInfo() {
  const fd = fs.openSync('/proc/self/mountinfo', fs.constants.O_RDONLY);
  try {
    const bytes = Buffer.alloc(mountInfoBound + 1);
    let length = 0;
    while (length < bytes.length) {
      const count = fs.readSync(fd, bytes, length, bytes.length - length, null);
      if (!count) break;
      length += count;
    }
    if (length > mountInfoBound) throw Error('provider mountinfo bound');
    return bytes.subarray(0, length).toString();
  } finally { fs.closeSync(fd); }
}

function requireSelection(unique, allocation) {
  if (!/^[a-f0-9]{32}$/.test(unique ?? '') || allocation.file !== `${directory}/provider-netns-${unique}` ||
      !/^net:\[[1-9][0-9]*\]$/.test(allocation.namespace ?? '') ||
      !Number.isSafeInteger(Number(allocation.namespace.slice(5, -1)))) throw Error('exact owned provider namespace required');
}

function requireLocation(allocation, io, uid) {
  const dir = io.lstatSync(directory);
  if (io.realpathSync(directory) !== directory || !dir.isDirectory() || dir.isSymbolicLink() || dir.uid !== uid ||
      (dir.mode & 0o777) !== 0o700 || !isDeepStrictEqual(allocation.directory,
        {path: directory, device: dir.dev, inode: dir.ino, uid: dir.uid, mode: dir.mode & 0o777}) ||
      io.realpathSync(allocation.file) !== allocation.file || io.readlinkSync('/proc/self/ns/mnt') !== allocation.mount_namespace) {
    throw Error('provider namespace location changed');
  }
}

export function createProviderNetnsAllocation(unique, namespace, {io = fs, uid = process.getuid()} = {}) {
  const file = `${directory}/provider-netns-${unique}`;
  requireSelection(unique, {file, namespace});
  const parent = io.lstatSync(directory), value = io.lstatSync(file);
  if (!value.isFile() || value.isSymbolicLink() || value.uid !== uid || value.nlink !== 1 ||
      (value.mode & 0o777) !== 0o600 || value.size !== 0) throw Error('owned empty namespace backing file required');
  const allocation = {file, namespace, mount_namespace: io.readlinkSync('/proc/self/ns/mnt'), backing_file: fileIdentity(value),
    directory: {path: directory, device: parent.dev, inode: parent.ino, uid: parent.uid, mode: parent.mode & 0o777}};
  requireLocation(allocation, io, uid);
  return allocation;
}

export function captureProviderNetnsMount(allocation, {io = fs, uid = process.getuid(), readMountInfo: read = readMountInfo} = {}) {
  const unique = allocation.file?.slice(`${directory}/provider-netns-`.length);
  requireSelection(unique, allocation);
  requireLocation(allocation, io, uid);
  const raw = read();
  if (typeof raw !== 'string' || Buffer.byteLength(raw) > mountInfoBound) throw Error('provider mountinfo bound');
  const lines = raw.split('\n').filter(line => line.split(' ')[4] === allocation.file);
  const value = io.lstatSync(allocation.file);
  if (!value.isFile() || value.isSymbolicLink() || value.uid !== uid) throw Error('provider namespace file replaced');
  if (!lines.length) {
    if (!isDeepStrictEqual(fileIdentity(value), allocation.backing_file)) throw Error('namespace backing file changed');
    return null;
  }
  if (lines.length !== 1) throw Error('ambiguous provider namespace mount');
  const [left, right, extra] = lines[0].split(' - '), fields = left.split(' '), tail = right?.split(' ');
  if (extra !== undefined || fields.length < 6 || !/^[1-9][0-9]*$/.test(fields[0]) || !/^[1-9][0-9]*$/.test(fields[1]) ||
      !/^\d+:\d+$/.test(fields[2]) || fields[3] !== allocation.namespace || tail?.[0] !== 'nsfs' || tail[1] !== 'nsfs' ||
      value.ino !== Number(allocation.namespace.slice(5, -1))) throw Error('provider namespace mount differs');
  return {mount_id: fields[0], parent_id: fields[1], device: fields[2], root: fields[3], target: fields[4],
    filesystem: tail[0], source: tail[1], file: fileIdentity(value)};
}

async function unmountOwned(file, group, beforeCommand) {
  const launcher = new OwnedLauncher({maxProcesses: 1, outputBytes: 65536});
  try {
    const child = launcher.launch('/run/wrappers/bin/umount', [file], {
      timeoutMs: 5000, graceMs: 1000, drainMs: 1000, outputBytes: 65536,
      env: {PATH: '/run/current-system/sw/bin:/usr/bin:/bin', LANG: 'C', TZ: 'UTC'},
      admitIdentity: identity => { beforeCommand(); admitOwnedWrapper(group, identity); },
    });
    const target = await Promise.race([child.targetStarted, child.closed.then(() => null)]);
    const closed = await child.closed;
    const physical = closed.drained ? await child.physicalClose : {code: null, signal: null, observed: false};
    const drain = await launcher.drain();
    return {wrapper: child.identity, target, closed, physical, drain, group: snapshotProviderCgroup(group)};
  } finally { await launcher.shutdown(); }
}

function requireStoppedUsers(record, group, readIdentity, snapshotGroup) {
  if (!Array.isArray(record.relay_processes) || record.relay_processes.some(value => value.drained !== true) ||
      !Array.isArray(record.provider_drain) || record.provider_drain.some(value => value.drained !== true) ||
      !Array.isArray(record.stop_results) || record.stop_results.some(value => value.stopped !== true)) throw Error('provider users not drained');
  requireDrainedCgroup(snapshotGroup(group).cgroup_events);
  const visit = value => {
    if (!value || typeof value !== 'object') return;
    if (Number.isSafeInteger(value.pid) && value.started && value.boot_id && sameProcessIdentity(value, readIdentity(value.pid))) {
      throw Error('owned provider namespace user remains live');
    }
    for (const nested of Object.values(value)) visit(nested);
  };
  visit(record);
}

export async function cleanupProviderNetns(record, group, hooks = {}) {
  const {readIdentity = readProcessIdentity, snapshotGroup = snapshotProviderCgroup, unmount = unmountOwned} = hooks;
  const proof = {path: record.namespace_mount?.file, status: 'failed', passed: false};
  try {
    if (!record.namespace_mount) return {status: 'not-created', passed: true};
    const allocation = record.namespace_mount;
    requireSelection(record.unique, allocation);
    requireStoppedUsers(record, group, readIdentity, snapshotGroup);
    const current = captureProviderNetnsMount(allocation, hooks);
    if (!current) return {...proof, status: 'not-mounted', passed: true};
    if (!isDeepStrictEqual(current, allocation.witness)) throw Error('retained provider mount identity changed');
    proof.before = current;
    const beforeCommand = () => {
      requireStoppedUsers(record, group, readIdentity, snapshotGroup);
      if (!isDeepStrictEqual(captureProviderNetnsMount(allocation, hooks), current)) throw Error('provider mount changed before target admission');
    };
    Object.assign(proof, await unmount(allocation.file, group, beforeCommand));
    if (!proof.target || proof.closed?.exit_code !== 0 || proof.closed.reason !== 'completed' || proof.closed.signal !== null || proof.closed.drained !== true ||
        proof.physical?.code !== 0 || proof.physical.signal !== null || !Array.isArray(proof.drain) ||
        proof.drain.length !== 1 || proof.drain.some(value => value.drained !== true || value.exit_code !== 0 || value.reason !== 'completed')) throw Error('provider namespace unmount did not drain');
    requireDrainedCgroup(snapshotGroup(group).cgroup_events);
    if (captureProviderNetnsMount(allocation, hooks) !== null) throw Error('provider namespace remains mounted');
    proof.status = 'unmounted';
    proof.passed = true;
  } catch (error) { proof.failure = error.message; }
  return proof;
}
