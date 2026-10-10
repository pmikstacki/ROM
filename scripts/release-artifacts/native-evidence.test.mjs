import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { prepareConsumerHost } from './native-preparation.mjs';
import { SQLITE_PROFILE } from './sqlite-profile.mjs';
import * as evidence from './native-evidence.mjs';

async function fixture(selectedProfile) {
  const parent = mkdtempSync(join(tmpdir(), 'rom-native-evidence-'));
  const root = join(parent, 'source'), directory = join(parent, 'preparation');
  mkdirSync(root); writeFileSync(join(root, 'Cargo.lock'), 'fixture lock');
  const witness = { revision: 'a'.repeat(40), tree: 'b'.repeat(40), source_inventory_sha256: 'c'.repeat(64), source_identity: 'd'.repeat(64), archive_sha256: 'e'.repeat(64), studio_sha256: 'f'.repeat(64), lock_sha256: createHash('sha256').update(readFileSync(join(root, 'Cargo.lock'))).digest('hex') };
  const prepared = await prepareConsumerHost({ root, witness, fence() {} }, directory, async (program, args, options) => {
    if (args[0] === 'build') {
      mkdirSync(join(options.env.CARGO_TARGET_DIR, 'debug'), { recursive: true });
      writeFileSync(join(options.env.CARGO_TARGET_DIR, 'debug/rom-recovery-host'), 'fixture executable', { mode: 0o755 });
    }
    return { code: 0, timedOut: false, stderr: '', stdout: args[0] === '--version' ? `${program} 1.99.0 (fixture)\n` : args[0] === 'test' ? `Rust-linked SQLite engine: 3.53.4\nRust-linked SQLite source ID: ${SQLITE_PROFILE.source_id}\nRust-linked SQLite compile options: COMPILER=gcc-13.3.0,ENABLE_COLUMN_METADATA,THREADSAFE=1\ntest sqlite_native_engine_matches_selected_build_profile ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n` : 'fixture compile complete\n' };
  }, selectedProfile);
  return { directory, record: prepared.record, expected: { witness, source_root: root, target_directory: join(directory, 'target'), binary_sha256: prepared.record.binary_sha256 } };
}

test('native evidence binds the full source witness, exact commands and retained log digests without execution', async () => {
  const { directory, record, expected } = await fixture();
  assert.equal(evidence.requireNativePreparation(record, expected, directory), record);
});

test('native evidence rejects substituted source, lock, binary or target identity', async () => {
  const { directory, record, expected } = await fixture();
  for (const mutate of [
    r => { r.head = 'b'.repeat(40); }, r => { r.verified_source.archive_sha256 = 'a'.repeat(64); },
    r => { r.files['Cargo.lock'] = 'a'.repeat(64); }, r => { r.binary_sha256 = 'a'.repeat(64); },
    r => { r.target_directory += '/other'; }, r => { r.source_root += '/other'; },
    r => { r.build_jobs = 3; }, r => { r.incremental = true; }, r => { r.source_unchanged = false; },
    r => { r.injected = true; }, r => { r.compile_exit_code = 1; },
  ]) {
    const changed = structuredClone(record); mutate(changed);
    assert.throws(() => evidence.requireNativePreparation(changed, expected, directory), /native preparation evidence/);
  }
});

test('native evidence rejects missing, reordered, failed, aborted or substituted commands', async () => {
  const { directory, record, expected } = await fixture();
  for (const mutate of [
    r => { r.commands.pop(); }, r => { r.commands.reverse(); },
    r => { r.commands[2].args.push('--release'); }, r => { r.commands[2].cwd = '/other'; },
    r => { r.commands[2].exit_code = 1; }, r => { r.commands[2].bounded_abort = true; },
    r => { r.commands[2].spawn_failed = true; }, r => { r.commands[2].started_at = 'not a date'; },
    r => { r.commands[2].stderr = '../outside.log'; },
  ]) {
    const changed = structuredClone(record); mutate(changed);
    assert.throws(() => evidence.requireNativePreparation(changed, expected, directory), /native preparation evidence/);
  }
});

test('native evidence detects changed log bytes and extra or missing digest entries', async () => {
  const { directory, record, expected } = await fixture();
  for (const mutate of [r => { delete r.log_sha256[r.commands[0].stdout]; }, r => { r.log_sha256.extra = 'a'.repeat(64); }]) {
    const changed = structuredClone(record); mutate(changed);
    assert.throws(() => evidence.requireNativePreparation(changed, expected, directory), /native preparation evidence/);
  }
  writeFileSync(join(directory, record.commands[2].stdout), 'substituted compile log');
  assert.throws(() => evidence.requireNativePreparation(record, expected, directory), /native preparation evidence/);
});

test('native evidence enforces a twenty-minute whole-preparation budget and real calendar timestamps', async () => {
  const { directory, record, expected } = await fixture();
  const bounded = structuredClone(record);
  bounded.commands.forEach((command, index) => {
    command.started_at = `2026-01-01T00:0${index}:00.000Z`;
    command.finished_at = command.started_at;
  });
  bounded.commands[2].finished_at = '2026-01-01T00:20:00.000Z';
  assert.equal(evidence.requireNativePreparation(bounded, expected, directory), bounded);
  const over = structuredClone(bounded);
  over.commands[2].finished_at = '2026-01-01T00:20:00.001Z';
  assert.throws(() => evidence.requireNativePreparation(over, expected, directory), /native preparation evidence/);
  const calendar = structuredClone(bounded);
  calendar.commands[2].started_at = '2026-02-31T02:00:00.000Z';
  calendar.commands[2].finished_at = '2026-02-31T03:00:00.000Z';
  assert.throws(() => evidence.requireNativePreparation(calendar, expected, directory), /native preparation evidence/);
});


test('schema-two native evidence requires exact selected public engine execution and explicit compile profile', async () => {
  const profile = {directory:'/owned/public-engine',record:{profile:SQLITE_PROFILE,library_sha256:'a'.repeat(64)}};
  const {directory,record,expected}=await fixture(profile);
  assert.equal(evidence.requireNativePreparation(record,expected,directory),record);
  for(const mutate of [r=>r.engine_verified=false,r=>r.engine_profile.profile.version='3.53.2',r=>r.compile_profile.dev_debug=true,r=>delete r.compile_profile,r=>r.commands.splice(2,1)]){
    const changed=structuredClone(record);mutate(changed);assert.throws(()=>evidence.requireNativePreparation(changed,expected,directory),/native preparation evidence/);
  }
  writeFileSync(join(directory,'evidence/02-engine.stdout.log'),'Rust-linked SQLite engine: 3.53.4\n');
  const changed=structuredClone(record);changed.log_sha256['evidence/02-engine.stdout.log']=createHash('sha256').update(readFileSync(join(directory,'evidence/02-engine.stdout.log'))).digest('hex');
  assert.throws(()=>evidence.requireNativePreparation(changed,expected,directory),/native preparation evidence/);
});
