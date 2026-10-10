import test from 'node:test';
import assert from 'node:assert/strict';
import { requireControlsBrowserReport, controlsScenarios } from './consumer-evidence.mjs';
import * as evidence from './consumer-evidence.mjs';

function report() {
  return {
    config: { projects: ['chromium', 'webkit'].map(name => ({ name })) }, errors: [],
    stats: { expected: 12, skipped: 0, unexpected: 0, flaky: 0 },
    suites: [{ specs: controlsScenarios.map(({ title, file }) => ({ title, file, ok: true, tests: ['chromium', 'webkit'].map(projectName => ({ projectName, expectedStatus: 'passed', status: 'expected', results: [{ status: 'passed', retry: 0, errors: [] }] })) })) }],
  };
}

test('control admission binds every required scenario to both actual report projects', () => {
  const result = requireControlsBrowserReport(report());
  assert.equal(result.executions, 12);
  assert.deepEqual(result.engines, ['chromium', 'webkit']);
});

test('missing, extra, renamed and wrong-file scenarios cannot satisfy browser admission', () => {
  for (const change of [
    r => r.suites[0].specs.pop(), r => r.suites[0].specs.push(structuredClone(r.suites[0].specs[0])),
    r => { r.suites[0].specs[0].title = 'unrelated passing test'; },
    r => { r.suites[0].specs[0].file = 'different.spec.ts'; },
    r => { r.suites = []; }, r => { r.config.projects.pop(); },
    r => { r.config.projects[0] = null; }, r => { r.suites[0].specs = {}; },
    r => { r.suites[0].suites = 'invalid'; },
    r => { r.suites[0].specs[0].tests.pop(); }, r => { r.suites[0].specs[0].tests[0].projectName = 'fake-engine'; },
    r => { r.stats.expected = 13; },
  ]) {
    const changed = report(); change(changed);
    assert.throws(() => requireControlsBrowserReport(changed), /browser evidence/);
  }
});

test('report errors, retries, skipped, failed, crashed and interrupted execution fail admission', () => {
  for (const change of [
    r => r.errors.push({ message: 'global error' }), r => { delete r.errors; },
    r => { r.stats.skipped = 1; }, r => { r.stats.flaky = 1; },
    r => { r.stats.unexpected = 1; }, r => { r.suites[0].specs[0].ok = false; },
    r => { r.suites[0].specs[0].tests[0].expectedStatus = 'skipped'; },
    r => { r.suites[0].specs[0].tests[0].status = 'unexpected'; },
    ...['skipped', 'failed', 'timedOut', 'interrupted'].map(status => r => { r.suites[0].specs[0].tests[0].results[0].status = status; }),
    r => { r.suites[0].specs[0].tests[0].results[0].retry = 1; },
    r => { r.suites[0].specs[0].tests[0].results.push({ status: 'passed', retry: 1, errors: [] }); },
    r => { r.suites[0].specs[0].tests[0].results[0].errors = [{ message: 'browser crash' }]; },
  ]) {
    const changed = report(); change(changed);
    assert.throws(() => requireControlsBrowserReport(changed), /browser evidence/);
  }
});

const recoveryTitles = [
  'lost acknowledgement, restart, IndexedDB reload and renewed session preserve A while retaining C',
  'current revocation and principal switch hide results and never retry another owner automatically',
  'invalid editor text survives reload and blocked navigation separately from the accepted command',
  'unrelated inventory editor composes the helper with its own validation and navigation policy',
  'principal change while a committed real HTTP result is withheld rejects late disclosure',
];
function recoveryReport() {
  const value = report();
  value.stats.expected = 10;
  value.suites[0].specs = recoveryTitles.map(title => ({ title, file: 'recovery.spec.ts', ok: true,
    tests: ['chromium', 'webkit'].map(projectName => ({ projectName, expectedStatus: 'passed', status: 'expected',
      results: [{ status: 'passed', retry: 0, errors: [] }] })) }));
  return value;
}
test('recovery admission requires all five lifecycle scenarios in both browser projects', () => {
  assert.equal(typeof evidence.requireRecoveryBrowserReport, 'function');
  const result = evidence.requireRecoveryBrowserReport(recoveryReport());
  assert.equal(result.executions, 10);
  assert.deepEqual(result.scenarios.map(item => item.title), recoveryTitles);
});
test('a missing late-principal scenario cannot pass recovery admission using a stale eight-case report', () => {
  assert.equal(typeof evidence.requireRecoveryBrowserReport, 'function');
  const value = recoveryReport();
  value.suites[0].specs.pop(); value.stats.expected = 8;
  assert.throws(() => evidence.requireRecoveryBrowserReport(value), /browser evidence/);
});
test('recovery admission rejects failed, retried, duplicated and mismatched lifecycle results', () => {
  assert.equal(typeof evidence.requireRecoveryBrowserReport, 'function');
  for (const change of [
    value => { value.suites[0].specs[0].file = 'public-root.spec.ts'; },
    value => { value.suites[0].specs[0].tests[0].results[0].status = 'timedOut'; },
    value => { value.suites[0].specs[0].tests[0].results[0].retry = 1; },
    value => { value.suites[0].specs[0].tests[0].results.push({ status: 'passed', retry: 1, errors: [] }); },
    value => { value.suites[0].specs[1] = structuredClone(value.suites[0].specs[0]); },
    value => { value.config.projects[1].name = 'chromium'; },
  ]) {
    const value = recoveryReport(); change(value);
    assert.throws(() => evidence.requireRecoveryBrowserReport(value), /browser evidence/);
  }
});
