import test from 'node:test';
import assert from 'node:assert/strict';
import * as evidence from './consumer-package.mjs';

function fixture(kind = 'controls') {
  const expected = { kind, source_input: '/verified/source/studio',
    source_files: { 'package.json': 'a'.repeat(64), 'LICENSE': 'b'.repeat(64), 'THIRD_PARTY_NOTICES.md': 'c'.repeat(64), 'src/index.ts': 'd'.repeat(64) },
    source_lock_sha256: 'e'.repeat(64), consumer_lock_sha256: 'f'.repeat(64),
    installed_package: '/evidence/consumer/node_modules/rom-studio', archive_sha256: '0'.repeat(64) };
  const record = { completed: true, source_mode: 'extracted_release_source', source_input: expected.source_input,
    source_files: structuredClone(expected.source_files), installed_source_matches: true,
    installed_package: expected.installed_package, archive_sha256: expected.archive_sha256,
    consumer_lock_sha256: expected.consumer_lock_sha256, browser_outcomes: {} };
  if (kind === 'controls') Object.assign(record, { source_lock_sha256: expected.source_lock_sha256,
    dependency_bootstrap: 'locked_npm_ci', browser_executed: true });
  else Object.assign(record, { producer_lock_sha256: expected.source_lock_sha256, admitted: true,
    adapter: kind.slice('recovery-'.length), head: null });
  return { record, expected };
}

test('consumer package metadata binds controls and both adapters to the independently selected package and locks', () => {
  for (const kind of ['controls', 'recovery-sqlite', 'recovery-redb']) {
    const { record, expected } = fixture(kind);
    assert.equal(evidence.requireConsumerPackage(record, expected), record);
  }
});

test('old or substituted package identity cannot pass from a completed browser claim', () => {
  for (const kind of ['controls', 'recovery-sqlite', 'recovery-redb']) {
    for (const mutate of [
      r => { r.source_input = '/other/studio'; }, r => { r.source_mode = 'authoring_checkout'; },
      r => { r.archive_sha256 = 'a'.repeat(64); }, r => { r.consumer_lock_sha256 = 'a'.repeat(64); },
      r => { r.source_files['src/index.ts'] = 'a'.repeat(64); }, r => { delete r.source_files.LICENSE; },
      r => { r.source_files['src/extra.ts'] = 'a'.repeat(64); },
      r => { r.installed_package = '/other/node_modules/rom-studio'; }, r => { r.installed_source_matches = false; },
      r => { r.completed = false; },
      r => { r[kind === 'controls' ? 'source_lock_sha256' : 'producer_lock_sha256'] = 'a'.repeat(64); },
      ...(kind === 'controls' ? [r => { r.browser_executed = false; }, r => { r.dependency_bootstrap = 'bootstrap_then_npm_ci'; }]
        : [r => { r.admitted = false; }, r => { r.adapter = kind.endsWith('sqlite') ? 'redb' : 'sqlite'; }, r => { r.head = 'a'.repeat(40); }]),
    ]) {
      const { record, expected } = fixture(kind); mutate(record);
      assert.throws(() => evidence.requireConsumerPackage(record, expected), /consumer package evidence/);
    }
  }
});

test('malformed independently supplied expectations are rejected rather than weakening binding', () => {
  for (const mutate of [
    e => { e.kind = 'anything'; }, e => { e.extra = true; }, e => { e.archive_sha256 = null; },
    e => { e.source_input = '/verified/../source/studio'; }, e => { e.source_files = {}; },
    e => { e.source_files['../outside'] = 'a'.repeat(64); },
    e => { e.source_files['src/index.ts'] = 'bad'; }, e => { delete e.source_files['THIRD_PARTY_NOTICES.md']; },
    e => { delete e.source_lock_sha256; },
  ]) {
    const { record, expected } = fixture(); mutate(expected);
    assert.throws(() => evidence.requireConsumerPackage(record, expected), /consumer package evidence/);
  }
});
