import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { syncBuiltinESMExports } from 'node:module';
import { recordedCommand } from './command-result.mjs';
import { produce } from './produce.mjs';
import { fixture } from './test-support.mjs';

function fullEvidenceStore(parent, logs = true) {
  const write = fs.writeFileSync;
  fs.writeFileSync = (path, ...args) => {
    if (String(path).startsWith(parent) && (String(path).endsWith('failure.json') || (logs && String(path).endsWith('.log')))) {
      throw Object.assign(Error('evidence store full'), { code: 'ENOSPC' });
    }
    return write(path, ...args);
  };
  syncBuiltinESMExports();
  return () => { fs.writeFileSync = write; syncBuiltinESMExports(); };
}

test('failed evidence write retains command outcome and bounded terminal diagnostics', async () => {
  const f = fixture(), restore = fullEvidenceStore(f.parent), reports = [];
  const errorOutput = console.error;
  console.error = message => reports.push(message);
  try {
    await assert.rejects(recordedCommand('gate', [], f.root, f.parent, 'gate-1', async () => ({
      code: 7, stdout: 'x'.repeat(100_000), stderr: 'original compiler failure', timedOut: false,
    })), error => {
      assert.equal(error.commandResult.exit_code, 7);
      assert.equal(error.commandResult.evidence_recorded, false);
      assert.match(error.message, /evidence store full/);
      return true;
    });
    const report = reports.map(message => JSON.parse(message)).find(record => record.type === 'release-command-evidence-failure');
    assert.equal(report.command.exit_code, 7);
    assert.equal(report.persistence_error.code, 'ENOSPC');
    assert.equal(report.stderr_tail, 'original compiler failure');
    assert.ok(Buffer.byteLength(report.stdout_tail) <= 8192);
  } finally { console.error = errorOutput; restore(); }
});

test('summary storage failure retains a normally recorded failed gate', async () => {
  const f = fixture(), restore = fullEvidenceStore(f.parent, false), reports = [];
  const errorOutput = console.error;
  console.error = message => reports.push(message);
  try {
    await assert.rejects(produce({ ...f, runner: async () => ({ code: 7, stdout: '', stderr: 'root cause', timedOut: false }) }), /release gate failed/);
    const report = reports.map(message => { try { return JSON.parse(message); } catch { return {}; } })
      .find(record => record.type === 'release-failure-evidence-failure');
    assert.match(report.original_error.message, /release gate failed/);
    assert.equal(report.last_command.exit_code, 7);
    assert.equal(fs.existsSync(f.output), false);
  } finally { console.error = errorOutput; restore(); }
});

test('failure summary write cannot replace the original gate failure', async () => {
  const f = fixture(), restore = fullEvidenceStore(f.parent), reports = [];
  const errorOutput = console.error;
  console.error = message => reports.push(message);
  try {
    await assert.rejects(produce({ ...f, runner: async () => ({ code: 7, stdout: '', stderr: 'root cause', timedOut: false }) }), error => {
      assert.equal(error.commandResult.exit_code, 7);
      assert.match(error.message, /evidence store full/);
      return true;
    });
    const report = reports.map(message => { try { return JSON.parse(message); } catch { return {}; } })
      .find(record => record.type === 'release-failure-evidence-failure');
    assert.equal(report.persistence_error.code, 'ENOSPC');
    assert.equal(report.failed_command.exit_code, 7);
    assert.equal(fs.existsSync(f.output), false);
  } finally { console.error = errorOutput; restore(); }
});
