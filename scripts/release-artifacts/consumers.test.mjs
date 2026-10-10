import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, cpSync, symlinkSync, renameSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { hash } from '../skills/files.mjs';
import { prepareConsumerHost } from './native-preparation.mjs';
import { gates, STUDIO_PROFILE } from './commands.mjs';
import { runConsumerGates } from './consumers.mjs';
import { packageFixture, recoveryFixtureEvidence } from './consumer-fixture.mjs';
import { controlsScenarios, recoveryScenarios } from './consumer-evidence.mjs';
import { recoveryHttpFixture } from './consumer-http-fixture.mjs';

// Simulated compiler and consumer commands test orchestration only.
async function fixture() {
  const parent = mkdtempSync(join(tmpdir(), 'rom-consumer-orchestration-'));
  const root = join(parent, 'extracted'), producer = join(parent, 'producer'), stage = join(parent, 'stage');
  for (const path of [root, producer, stage, join(stage, 'evidence')]) mkdirSync(path);
  writeFileSync(join(root, 'Cargo.lock'), 'fixture-lock');
  const witness = { revision: 'a'.repeat(40), tree: 'b'.repeat(40), source_inventory_sha256: 'c'.repeat(64), archive_sha256: 'd'.repeat(64), studio_sha256: 'e'.repeat(64), lock_sha256: hash(join(root, 'Cargo.lock')), source_identity: 'f'.repeat(64) };
  const lease = { root, witness, fence() { if (hash(join(root, 'Cargo.lock')) !== witness.lock_sha256) throw Error('source drift'); } };
  const directory = join(parent, 'native');
  const prepared = await prepareConsumerHost(lease, directory, async (program, args, options) => {
    if (args[0] === 'build') {
      mkdirSync(join(options.env.CARGO_TARGET_DIR, 'debug'), { recursive: true });
      writeFileSync(join(options.env.CARGO_TARGET_DIR, 'debug/rom-recovery-host'), 'fixture', { mode: 0o755 });
    }
    return { code: 0, stdout: `${program} 1.99.0 (fixture)`, stderr: '', timedOut: false };
  });
  const assets = join(parent, 'assets');
  const results = gates(producer, assets, STUDIO_PROFILE).map(([program, args]) => ({ program, args, cwd: producer, exit_code: 0, bounded_abort: false, spawn_failed: false }));
  const context = { extracted_root: root, evidence_root: join(parent, 'consumer-evidence'), host_binary: prepared.binary,
    revision: witness.revision, tree: witness.tree, source_inventory_sha256: witness.source_inventory_sha256, studio_sha256: witness.studio_sha256, archive_sha256: witness.archive_sha256 };
  return { lease, options: { root: producer, stage, assets, results, context, native: { directory, record: prepared.record } } };
}
const passed = async () => ({ code: 0, stdout: 'fixture consumer', stderr: '', timedOut: false });

function retainedReports(f, args) {
  const directory = args[1], kind = directory.split('/').at(-1);
  const packageEvidence = packageFixture(kind, { root: f.lease.root, directory });
  if (kind !== 'controls') {
    Object.assign(packageEvidence.record, recoveryFixtureEvidence(f.lease.root, directory));
    packageEvidence.record.http = recoveryHttpFixture(kind.slice(9));
    writeFileSync(join(directory, 'http-results.json'), JSON.stringify(packageEvidence.record.http));
    const provenance = join(f.options.native.directory, 'target/debug/source.json');
    cpSync(provenance, join(directory, 'native-provenance.json'));
    Object.assign(packageEvidence.record, {
      native_binary: f.options.context.host_binary, native_binary_sha256: hash(f.options.context.host_binary),
      native_compile_lock_sha256: f.lease.witness.lock_sha256,
      current_root_lock_sha256: f.lease.witness.lock_sha256, native_provenance_sha256: hash(provenance),
    });
  }
  const scenarios = kind === 'controls' ? controlsScenarios : recoveryScenarios;
  const browser = { config: { projects: ['chromium', 'webkit'].map(name => ({ name })) }, errors: [],
    stats: { expected: scenarios.length * 2, skipped: 0, unexpected: 0, flaky: 0 },
    suites: [{ specs: scenarios.map(({ title, file }) => ({ title, file, ok: true,
      tests: ['chromium', 'webkit'].map(projectName => ({ projectName, expectedStatus: 'passed', status: 'expected',
        results: [{ status: 'passed', retry: 0, errors: [] }] })) })) }] };
  writeFileSync(join(directory, 'result.json'), JSON.stringify(packageEvidence.record));
  writeFileSync(join(directory, 'browser-results.json'), JSON.stringify(browser));
}

test('consumer execution uses the prepared source-bound host and fixed ordered gates', async () => {
  const f = await fixture(), calls = [];
  await runConsumerGates(f.lease, f.options, async (...args) => { calls.push(args); retainedReports(f, args[1]); return passed(); });
  assert.equal(calls.length, 3);
  assert.ok(calls.every(([, , options]) => options.timeout === 1_200_000 && options.maxBytes === 32 * 1024 * 1024));
  assert.equal(f.options.results.length, 11);
  assert.ok(calls.every(([, args]) => args[0].startsWith(f.lease.root + '/studio/tests/')));
  assert.match(readFileSync(join(f.options.stage, 'evidence/gate-11.stdout.log'), 'utf8'), /fixture consumer/);
});

test('substituted source, binary or incomplete preceding gates execute no consumer', async () => {
  for (const mutate of [
    f => { f.options.context.archive_sha256 = '0'.repeat(64); },
    f => { writeFileSync(f.options.context.host_binary, 'replacement'); },
    f => { f.options.native.record.verified_source.lock_sha256 = '0'.repeat(64); },
    f => { f.options.results.pop(); },
  ]) {
    const f = await fixture(); mutate(f); let calls = 0;
    await assert.rejects(runConsumerGates(f.lease, f.options, async () => { calls++; return passed(); }));
    assert.equal(calls, 0);
  }
});

test('source mutation after one consumer stops subsequent execution and retains its failure log', async () => {
  const f = await fixture(); let calls = 0;
  await assert.rejects(runConsumerGates(f.lease, f.options, async () => {
    calls++; writeFileSync(join(f.lease.root, 'Cargo.lock'), 'changed'); return passed();
  }));
  assert.equal(calls, 1);
  assert.equal(f.options.results.length, 9);
  assert.equal(f.options.results[8].spawn_failed, false);
  assert.equal(f.options.results[8].exit_code, 0);
  assert.match(readFileSync(join(f.options.stage, 'evidence/gate-9.stdout.log'), 'utf8'), /fixture consumer/);
  assert.match(readFileSync(join(f.options.stage, 'evidence/consumer-failure.json'), 'utf8'), /source drift/);
});

test('failed or bounded-aborted consumer stops the sequence without completion', async () => {
  for (const outcome of [{ code: 1, timedOut: false }, { code: 0, timedOut: true }]) {
    const f = await fixture(); let calls = 0;
    await assert.rejects(runConsumerGates(f.lease, f.options, async () => { calls++; return { ...outcome, stdout: '', stderr: 'failed' }; }));
    assert.equal(calls, 1); assert.equal(f.options.results.length, 9);
  }
});

test('occupied or linked output roots execute no consumer and preserve existing bytes', async () => {
  for (const linked of [false, true]) {
    const f = await fixture(); let calls = 0;
    if (linked) {
      const parent = join(f.options.stage, 'linked'); symlinkSync(f.lease.root, parent);
      f.options.context.evidence_root = join(parent, 'generated');
    } else {
      mkdirSync(f.options.context.evidence_root);
      writeFileSync(join(f.options.context.evidence_root, 'retained'), 'prior evidence');
    }
    await assert.rejects(runConsumerGates(f.lease, f.options, async () => { calls++; return passed(); }));
    assert.equal(calls, 0);
    if (!linked) assert.equal(readFileSync(join(f.options.context.evidence_root, 'retained'), 'utf8'), 'prior evidence');
  }
});

test('failure-summary storage errors preserve the original failure and existing evidence', async () => {
  const f = await fixture(), path = join(f.options.stage, 'evidence/consumer-failure.json');
  writeFileSync(path, 'prior failure evidence');
  const diagnostics = [], previous = console.error;
  console.error = message => diagnostics.push(JSON.parse(message));
  try {
    await assert.rejects(runConsumerGates(f.lease, f.options, async () => ({ code: 1, stdout: '', stderr: 'consumer rejected', timedOut: false })), /release gate failed/);
  } finally { console.error = previous; }
  assert.equal(readFileSync(path, 'utf8'), 'prior failure evidence');
  assert.equal(diagnostics.length, 1);
  assert.equal(diagnostics[0].type, 'consumer-failure-evidence-unavailable');
  assert.equal(diagnostics[0].persistence_error.code, 'EEXIST');
  assert.match(readFileSync(join(f.options.stage, 'evidence/gate-9.stderr.log'), 'utf8'), /consumer rejected/);
});

test('linked command log directory cannot write into extracted source', async () => {
  const f = await fixture(); let calls = 0;
  renameSync(join(f.options.stage, 'evidence'), join(f.options.stage, 'retained-evidence-directory'));
  symlinkSync(f.lease.root, join(f.options.stage, 'evidence'));
  await assert.rejects(runConsumerGates(f.lease, f.options, async () => { calls++; return passed(); }));
  assert.equal(calls, 0);
});

test('three successful command exits cannot admit absent consumer reports', async () => {
  const f = await fixture();
  await assert.rejects(runConsumerGates(f.lease, f.options, passed), /consumer evidence/);
  assert.equal(f.options.results.length, 11);
  assert.match(readFileSync(join(f.options.stage, 'evidence/consumer-failure.json'), 'utf8'), /consumer evidence/);
});

test('successful exits preserve logs but cannot hide wrong browser scenarios or changed installed bytes', async () => {
  for (const altered of ['browser', 'package']) {
    const f = await fixture();
    await assert.rejects(runConsumerGates(f.lease, f.options, async (_program, args) => {
      retainedReports(f, args);
      if (args[1].endsWith('/controls')) {
        const directory = args[1];
        if (altered === 'browser') {
          const path = join(directory, 'browser-results.json'), report = JSON.parse(readFileSync(path, 'utf8'));
          report.suites[0].specs[0].file = 'unrelated.spec.ts';
          writeFileSync(path, JSON.stringify(report));
        } else {
          const path = join(directory, 'consumer/node_modules/rom-studio/src/index.ts');
          writeFileSync(path, 'substituted installed package');
          const reportPath = join(directory, 'result.json'), report = JSON.parse(readFileSync(reportPath, 'utf8'));
          report.source_files['src/index.ts'] = hash(path);
          writeFileSync(reportPath, JSON.stringify(report));
        }
      }
      return passed();
    }), /consumer evidence/);
    assert.equal(f.options.results.length, 11);
    assert.ok(f.options.results.every(record => record.exit_code === 0 && record.spawn_failed === false));
    assert.match(readFileSync(join(f.options.stage, 'evidence/gate-11.stdout.log'), 'utf8'), /fixture consumer/);
    assert.match(readFileSync(join(f.options.stage, 'evidence/consumer-failure.json'), 'utf8'), /consumer evidence/);
  }
});

test('a recovery report cannot substitute a different binary or native compile lock', async () => {
  for (const field of ['native_binary', 'native_binary_sha256', 'native_compile_lock_sha256', 'current_root_lock_sha256', 'native_provenance_sha256']) {
    const f = await fixture();
    await assert.rejects(runConsumerGates(f.lease, f.options, async (_program, args) => {
      retainedReports(f, args);
      if (args[1].endsWith('/recovery-sqlite')) {
        const path = join(args[1], 'result.json'), report = JSON.parse(readFileSync(path, 'utf8'));
        report[field] = field === 'native_binary' ? '/other/host' : '0'.repeat(64);
        writeFileSync(path, JSON.stringify(report));
      }
      return passed();
    }), /consumer evidence/);
    assert.equal(f.options.results.length, 11);
  }
});

test('successful consumers cannot omit recovery fixture source identity', async () => {
  const f = await fixture();
  await assert.rejects(runConsumerGates(f.lease, f.options, async (_program, args) => {
    retainedReports(f, args);
    if (args[1].endsWith('/recovery-sqlite')) {
      const path = join(args[1], 'result.json'), report = JSON.parse(readFileSync(path));
      delete report.fixture_source_sha256;
      writeFileSync(path, JSON.stringify(report));
    }
    return passed();
  }), /consumer evidence/);
});

test('successful commands cannot hide missing or changed retained HTTP recovery evidence', async () => {
  for (const changed of [false, true]) {
    const f = await fixture();
    await assert.rejects(runConsumerGates(f.lease, f.options, async (_program, args) => {
      retainedReports(f, args);
      if (args[1].endsWith('/recovery-sqlite')) {
        const path = join(args[1], 'result.json'), report = JSON.parse(readFileSync(path));
        if (changed) {
          report.http.counts.replayed.counts[2] = 3;
          writeFileSync(join(args[1], 'http-results.json'), JSON.stringify(report.http));
        } else delete report.http;
        writeFileSync(path, JSON.stringify(report));
      }
      return passed();
    }), /consumer evidence/);
    assert.equal(f.options.results.length, 11);
  }
});
