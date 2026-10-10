import { SQLITE_PROFILE } from './sqlite-profile.mjs';
import test, { mock } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import * as native from './native-preparation.mjs';

function fixture() {
  const parent = mkdtempSync(join(tmpdir(), 'rom-native-preparation-'));
  const root = join(parent, 'source'); mkdirSync(root);
  writeFileSync(join(root, 'Cargo.lock'), 'frozen fixture lock');
  const witness = { revision: 'a'.repeat(40), tree: 'b'.repeat(40), source_inventory_sha256: 'c'.repeat(64), archive_sha256: 'd'.repeat(64), studio_sha256: 'e'.repeat(64), lock_sha256: 'b83265d7a3f0ad8a230b80de71546d2c4f13c43f4c0d8c865501a4ace25e9a51', source_identity: 'f'.repeat(64) };
  witness.lock_sha256 = createHash('sha256').update(readFileSync(join(root, 'Cargo.lock'))).digest('hex');
  let fences = 0;
  return { parent, root, witness, fence: () => { fences++; }, fences: () => fences };
}

test('preparation binds external target and successful compile to the supplied source lease', async () => {
  const lease = fixture(), evidence = join(lease.parent, 'native');
  const calls = [];
  const runner = async (program, args, options) => {
    calls.push({ program, args, options });
    if (calls.length === 1) await new Promise(resolve => setTimeout(resolve, 25));
    if (args[0] === 'build') {
      mkdirSync(join(options.env.CARGO_TARGET_DIR, 'debug'), { recursive: true });
      writeFileSync(join(options.env.CARGO_TARGET_DIR, 'debug/rom-recovery-host'), 'fixture executable', { mode: 0o755 });
    }
    return { code: 0, stdout: `${program} fixture version`, stderr: '', timedOut: false };
  };
  const result = await native.prepareConsumerHost(lease, evidence, runner);
  assert.equal(lease.fences(), 2);
  assert.equal(calls.length, 3);
  assert.ok(calls[0].options.timeout <= 1_200_000);
  assert.ok(calls[1].options.timeout < calls[0].options.timeout);
  assert.equal(calls[0].options.maxBytes, 32 * 1024 * 1024);
  assert.deepEqual(calls[2].args, ['build', '--locked', '-p', 'rom-recovery-host']);
  assert.equal(calls[2].options.cwd, lease.root);
  assert.equal(calls[2].options.env.CARGO_INCREMENTAL, '0');
  assert.equal(calls[2].options.env.CARGO_BUILD_JOBS, '2');
  assert.equal(calls[2].options.env.CARGO_TARGET_DIR, join(evidence, 'target'));
  const record = JSON.parse(readFileSync(result.provenance, 'utf8'));
  assert.deepEqual(record.verified_source, lease.witness);
  assert.equal(record.head, lease.witness.revision);
  assert.equal(record.compile_exit_code, 0);
  assert.match(record.binary_sha256, /^[a-f0-9]{64}$/);
});

test('source mutation fence failure never emits successful provenance', async () => {
  const lease = fixture(), evidence = join(lease.parent, 'native');
  lease.fence = () => { if (existsSync(join(evidence, 'target/debug/rom-recovery-host'))) throw Error('source drift'); };
  const runner = async (_program, args, options) => {
    if (args[0] === 'build') {
      mkdirSync(join(options.env.CARGO_TARGET_DIR, 'debug'), { recursive: true });
      writeFileSync(join(options.env.CARGO_TARGET_DIR, 'debug/rom-recovery-host'), 'fixture', { mode: 0o755 });
    }
    return { code: 0, stdout: '', stderr: '', timedOut: false };
  };
  await assert.rejects(native.prepareConsumerHost(lease, evidence, runner), /source drift/);
  assert.equal(existsSync(join(evidence, 'target/debug/source.json')), false);
});

test('failed or bounded-aborted compiler retains logs without successful provenance', async () => {
  for (const outcome of [{ code: 1, timedOut: false }, { code: 0, timedOut: true }]) {
    const lease = fixture(), evidence = join(lease.parent, 'native');
    await assert.rejects(native.prepareConsumerHost(lease, evidence, async () => ({ ...outcome, stdout: '', stderr: 'compiler failure' })), /native preparation/);
    assert.equal(existsSync(join(evidence, 'evidence/00-rustc.stderr.log')), true);
    assert.equal(existsSync(join(evidence, 'target/debug/source.json')), false);
  }
});

test('an exhausted preparation deadline prevents the next compiler command and retains failure evidence', async () => {
  const lease = fixture(), evidence = join(lease.parent, 'native');
  let now = 100, calls = 0;
  const clock = mock.method(performance, 'now', () => now);
  try {
    await assert.rejects(native.prepareConsumerHost(lease, evidence, async () => {
      calls++; now += 1_200_001;
      return { code: 0, stdout: 'rustc fixture', stderr: '', timedOut: false };
    }), /native preparation command failed/);
    assert.equal(calls, 1);
    const failure = JSON.parse(readFileSync(join(evidence, 'failure.json'), 'utf8'));
    assert.equal(failure.commands.length, 2);
    assert.equal(failure.commands[0].exit_code, 0);
    assert.equal(failure.commands[1].bounded_abort, true);
    assert.match(readFileSync(join(evidence, failure.commands[1].stderr), 'utf8'), /deadline exceeded/);
    assert.equal(existsSync(join(evidence, 'target/debug/source.json')), false);
  } finally { clock.mock.restore(); }
});

test('preparation rejects outputs inside source and refuses existing evidence', async () => {
  const lease = fixture(); let calls = 0;
  const runner = async () => { calls++; throw Error('must not run'); };
  await assert.rejects(native.prepareConsumerHost(lease, join(lease.root, 'generated'), runner), /native preparation/);
  const evidence = join(lease.parent, 'existing'); mkdirSync(evidence);
  await assert.rejects(native.prepareConsumerHost(lease, evidence, runner), /EEXIST/);
  assert.equal(calls, 0);
});

test('native preparation rejects a changed lock even when the supplied fence omits that check', async () => {
  const lease = fixture(), evidence = join(lease.parent, 'native');
  const runner = async (_program, args, options) => {
    if (args[0] === 'build') {
      mkdirSync(join(options.env.CARGO_TARGET_DIR, 'debug'), { recursive: true });
      writeFileSync(join(options.env.CARGO_TARGET_DIR, 'debug/rom-recovery-host'), 'fixture', { mode: 0o755 });
      writeFileSync(join(lease.root, 'Cargo.lock'), 'replaced lock');
    }
    return { code: 0, stdout: '', stderr: '', timedOut: false };
  };
  await assert.rejects(native.prepareConsumerHost(lease, evidence, runner), /lock mismatch/);
  assert.equal(existsSync(join(evidence, 'target/debug/source.json')), false);
});

test('native preparation refuses a substituted symlink or nonexecutable binary', async () => {
  for (const symbolic of [true, false]) {
    const lease = fixture(), evidence = join(lease.parent, 'native');
    const runner = async (_program, args, options) => {
      if (args[0] === 'build') {
        mkdirSync(join(options.env.CARGO_TARGET_DIR, 'debug'), { recursive: true });
        const binary = join(options.env.CARGO_TARGET_DIR, 'debug/rom-recovery-host');
        if (symbolic) symlinkSync(join(lease.root, 'Cargo.lock'), binary);
        else writeFileSync(binary, 'not executable', { mode: 0o600 });
      }
      return { code: 0, stdout: '', stderr: '', timedOut: false };
    };
    await assert.rejects(native.prepareConsumerHost(lease, evidence, runner), /private executable/);
    assert.equal(existsSync(join(evidence, 'target/debug/source.json')), false);
  }
});

test('failure metadata refusal preserves the original command failure and fallback diagnostic', async () => {
  const lease = fixture(), evidence = join(lease.parent, 'native');
  const diagnostics = [], original = console.error;
  console.error = text => diagnostics.push(JSON.parse(text));
  try {
    await assert.rejects(native.prepareConsumerHost(lease, evidence, async () => {
      mkdirSync(join(evidence, 'failure.json'));
      return { code: 1, stdout: '', stderr: 'compiler failure', timedOut: false };
    }), /native preparation command failed/);
    assert.equal(diagnostics.length, 1);
    assert.equal(diagnostics[0].type, 'native-preparation-evidence-failure');
    assert.equal(diagnostics[0].last_command.exit_code, 1);
    assert.equal(diagnostics[0].persistence_error.code, 'EEXIST');
  } finally { console.error = original; }
});

test('public profile runs real linked-engine test and records schema2 rather than inheriting private native overrides', async () => {
 const lease=fixture(), directory=join(lease.parent,'native-profile'), calls=[];
 const profile={directory:'/public/engine',record:{profile:{version:'3.53.4'},library_sha256:'a'.repeat(64)},environment:{ROM_EXPECT_SQLITE_VERSION:'3.53.4',PKG_CONFIG_PATH:'/public/engine/lib/pkgconfig'}};
 const runner=async(program,args,options)=>{calls.push({program,args,options});if(args[0]==='build'){mkdirSync(join(options.env.CARGO_TARGET_DIR,'debug'),{recursive:true});writeFileSync(join(options.env.CARGO_TARGET_DIR,'debug/rom-recovery-host'),'fixture executable',{mode:0o755});}return{code:0,stdout:args[0]==='test'?`Rust-linked SQLite engine: 3.53.4\nRust-linked SQLite source ID: ${SQLITE_PROFILE.source_id}\nRust-linked SQLite compile options: COMPILER=gcc-13.3.0,ENABLE_COLUMN_METADATA,THREADSAFE=1\ntest sqlite_native_engine_matches_selected_build_profile ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n`:`${program} fixture version`,stderr:'',timedOut:false};};
 const result=await native.prepareConsumerHost(lease,directory,runner,profile);assert.equal(calls.length,4);assert.equal(calls[2].args[0],'test');assert.equal(result.record.schema_version,2);assert.equal(result.record.engine_verified,true);assert.equal(result.record.engine_profile.library_sha256,'a'.repeat(64));assert.equal(calls[2].options.env.ROM_EXPECT_SQLITE_VERSION,'3.53.4');assert.equal(calls[2].options.env.CARGO_PROFILE_DEV_DEBUG,'0');assert.equal(calls[2].options.env.CARGO_PROFILE_TEST_DEBUG,'0');assert.deepEqual(result.record.compile_profile,{dev_debug:false,test_debug:false});
});


test('selected-profile native build materializes a private binary without mutating the Cargo dependency hardlink',async()=>{
 const {linkSync,lstatSync}=await import('node:fs');const lease=fixture(),directory=join(lease.parent,'native-linked');
 const profile={directory:'/public/engine',record:{profile:{version:'3.53.4'},library_sha256:'a'.repeat(64)}};
 let dependency;
 const runner=async(program,args,options)=>{
   if(args[0]==='build'){
     const target=options.env.CARGO_TARGET_DIR;mkdirSync(join(target,'debug/deps'),{recursive:true});dependency=join(target,'debug/deps/rom-recovery-host-hash');
     writeFileSync(dependency,'original native executable',{mode:0o755});linkSync(dependency,join(target,'debug/rom-recovery-host'));
   }
   return{code:0,timedOut:false,stderr:'',stdout:args[0]==='test'?`Rust-linked SQLite engine: 3.53.4\nRust-linked SQLite source ID: ${SQLITE_PROFILE.source_id}\nRust-linked SQLite compile options: COMPILER=gcc-13.3.0,ENABLE_COLUMN_METADATA,THREADSAFE=1\ntest sqlite_native_engine_matches_selected_build_profile ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n`:program+' 1.99.0 (fixture)\n'};
 };
 const result=await native.prepareConsumerHost(lease,directory,runner,profile);
 assert.equal(lstatSync(result.binary).nlink,1);assert.notEqual(lstatSync(result.binary).ino,lstatSync(dependency).ino);
 assert.equal(readFileSync(dependency,'utf8'),'original native executable');assert.equal(readFileSync(result.binary,'utf8'),'original native executable');
});
