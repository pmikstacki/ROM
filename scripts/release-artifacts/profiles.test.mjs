import test from 'node:test';
import assert from 'node:assert/strict';
import { gates, gatePhase, requireCompleteGate, verifyCommands } from './commands.mjs';
import { mkdirSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const profile = 'rom-public-consumers-v3';
const root = '/producer/source', assets = '/stage/studio-assets';
const context = {
  extracted_root: '/stage/verified-source', evidence_root: '/stage/consumer-evidence',
  host_binary: '/stage/native-target/debug/rom-recovery-host',
  revision: 'a'.repeat(40), tree: 'b'.repeat(40),
  source_inventory_sha256: 'c'.repeat(64), studio_sha256: 'd'.repeat(64), archive_sha256: 'e'.repeat(64),
};
const records = commands => commands.map(([program, args]) => ({ program, args, cwd: root, exit_code: 0, bounded_abort: false, spawn_failed: false }));

test('public consumer profile retains eight historical commands then adds three exact source-bound consumers', () => {
  const selected = gates(root, assets, profile, context);
  assert.deepEqual(selected.slice(0, 8), gates(root, assets, 'rom-studio-v2'));
  assert.equal(selected.length, 11);
  assert.deepEqual(selected.slice(8), [
    ['node', ['/stage/verified-source/studio/tests/public-controls/verify.mjs', '/stage/consumer-evidence/controls', '--source-dir', '/stage/verified-source/studio', '--admit']],
    ['node', ['/stage/verified-source/studio/tests/mutation-recovery/verify.mjs', '/stage/consumer-evidence/recovery-sqlite', '--source-dir', '/stage/verified-source/studio', '--adapter', 'sqlite', '--host-binary', context.host_binary, '--port', '43282', '--admit']],
    ['node', ['/stage/verified-source/studio/tests/mutation-recovery/verify.mjs', '/stage/consumer-evidence/recovery-redb', '--source-dir', '/stage/verified-source/studio', '--adapter', 'redb', '--host-binary', context.host_binary, '--port', '43283', '--admit']],
  ]);
  requireCompleteGate(records(selected), profile, assets, context);
});

test('consumer gates reject missing, additional, reordered and substituted executions', () => {
  const selected = gates(root, assets, profile, context);
  for (const mutate of [
    rows => rows.pop(), rows => rows.push(structuredClone(rows[10])),
    rows => { [rows[8], rows[9]] = [rows[9], rows[8]]; },
    rows => { rows[8].args[0] = '/producer/source/studio/tests/public-controls/verify.mjs'; },
    rows => { rows[9].args[5] = 'redb'; },
    rows => { rows[10].args[9] = '43282'; },
    rows => { rows[8].cwd = '/other/source'; },
    rows => { rows[10].bounded_abort = true; },
  ]) {
    const changed = records(selected); mutate(changed);
    assert.throws(() => requireCompleteGate(changed, profile, assets, context), /verification gate/);
  }
});

test('consumer context rejects missing, unknown, noncanonical paths and inconsistent identities', () => {
  for (const key of Object.keys(context)) {
    const changed = structuredClone(context); delete changed[key];
    assert.throws(() => gates(root, assets, profile, changed), /consumer context/);
  }
  for (const changed of [
    { ...context, injected: true }, { ...context, extracted_root: '/stage/../source' },
    { ...context, evidence_root: 'relative' }, { ...context, host_binary: '/stage/native-target/debug/../rom-recovery-host' },
    { ...context, evidence_root: '/stage/verified-source/evidence' },
    { ...context, host_binary: '/stage/verified-source/target/rom-recovery-host' },
    { ...context, tree: 'b'.repeat(64) }, { ...context, revision: 'invalid' },
    { ...context, studio_sha256: 'not-a-digest' },
  ]) assert.throws(() => gates(root, assets, profile, changed), /consumer context/);
  assert.throws(() => gates(root, assets, profile), /consumer context/);
});

test('historical profiles reject added consumer claims and retain their exact gate counts', () => {
  assert.equal(gates(root, assets, 'native-source-v1').length, 6);
  assert.equal(gates(root, assets, 'rom-studio-v2').length, 8);
  assert.throws(() => requireCompleteGate(records(gates(root, assets, 'rom-studio-v2')), profile, assets, context), /verification gate/);
  assert.throws(() => requireCompleteGate(records(gates(root, assets, profile, context)), 'rom-studio-v2', assets), /verification gate/);
});

test('public consumer phases are explicit and preserve the original ordered execution boundary', () => {
  const selected = gates(root, assets, profile, context);
  assert.deepEqual(gatePhase(root, assets, profile, context, 'prepare'), selected.slice(0, 7));
  assert.deepEqual(gatePhase(root, assets, profile, context, 'assets'), selected.slice(7, 8));
  assert.deepEqual(gatePhase(root, assets, profile, context, 'consumers'), selected.slice(8));
  for (const phase of [undefined, 'all', 'unknown']) assert.throws(() => gatePhase(root, assets, profile, context, phase), /gate phase/);
});

test('explicit native execution runs all six native gates without a nonexistent asset phase', async () => {
  const stage = mkdtempSync(join(tmpdir(), 'rom-native-gate-phase-'));
  mkdirSync(join(stage, 'evidence'));
  const results = [], executed = [];
  const runner = async (program, args) => {
    executed.push([program, args]);
    return { code: 0, stdout: '', stderr: '', timedOut: false, spawn_failed: false };
  };
  await verifyCommands(root, stage, results, runner, undefined, { profile: 'native-source-v1' });
  assert.deepEqual(executed, gates(root, undefined, 'native-source-v1'));
  requireCompleteGate(results, 'native-source-v1');
});
