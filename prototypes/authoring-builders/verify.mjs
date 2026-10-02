#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync, readdirSync, statSync } from 'node:fs';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';
process.chdir(fileURLToPath(new URL('.', import.meta.url)));
mkdirSync('evidence', { recursive: true });
const run = (args, options = {}) => {
  const r = spawnSync(args[0], args.slice(1), { encoding: 'utf8', maxBuffer: 8e6, ...options });
  if (r.error) throw r.error;
  return r;
};
const pass = (args) => {
  const r = run(args);
  if (r.status !== 0) throw new Error(`${args.join(' ')}\n${r.stdout}\n${r.stderr}`);
  return r.stdout + r.stderr;
};
const observations = [pass(['rustc', '--version']), pass(['cargo', '--version'])];
pass(['cargo', 'fmt', '--check']);
pass(['cargo', 'clippy', '--locked', '--all-targets', '--all-features', '--', '-D', 'warnings']);
observations.push(pass(['cargo', 'run', '--locked', '--quiet']));
const failCases = [
  ['bon_missing', /name|Name/], ['bon_duplicate', /name|Name/], ['bon_wrong_type', /u32/],
  ['bon_action_missing', /expected_revision|ExpectedRevision/], ['bon_conditional', /mismatched types/],
  ['typed_missing', /name|Name/], ['typed_duplicate', /name|Name/], ['typed_wrong_type', /u32/],
  ['typed_action_missing', /expected_revision|ExpectedRevision/], ['typed_conditional', /mismatched types/],
  ['manual_wrong_type', /u32/], ['typed_strip_none', /String/],
];
for (const [name, relevant] of failCases) {
  const r = run(['cargo', 'check', '--locked', '--manifest-path', 'fixtures/Cargo.toml', '--bin', name, '--message-format=json']);
  const messages = r.stdout.split('\n').filter(Boolean).map(line => JSON.parse(line)).filter(m => m.reason === 'compiler-message' && m.target.name === name);
  const errors = messages.filter(m => m.message.level === 'error');
  const rendered = messages.map(m => m.message.rendered).join('');
  if (r.status === 0 || errors.length === 0 || !relevant.test(rendered)) {
    throw new Error(`Expected fixture-specific compile error for ${name}\n${r.stdout}\n${r.stderr}`);
  }
  writeFileSync(`evidence/${name}.txt`, rendered);
  observations.push(`PASS expected compile failure ${name}: ${errors.map(m => m.message.code?.code ?? 'no-code').join(', ')}\n`);
}
for (const name of ['manual_missing', 'manual_action_missing', 'manual_duplicate', 'manual_conditional']) {
  pass(['cargo', 'run', '--locked', '--quiet', '--manifest-path', 'fixtures/Cargo.toml', '--bin', name]);
  if (name === 'manual_conditional') pass(['cargo', 'run', '--locked', '--quiet', '--manifest-path', 'fixtures/Cargo.toml', '--bin', name, '--', 'enabled']);
  observations.push(`PASS runtime control ${name}\n`);
}
const bytes = path => readdirSync(path).reduce((sum, entry) => {
  const full = `${path}/${entry}`; const info = statSync(full);
  return sum + (info.isDirectory() ? bytes(full) : info.size);
}, 0);
// Unique directories ensure fresh local compilation without removing any prior evidence.
// Downloaded source cache remains warm; this is NOT a cold-machine benchmark.
const stamp = Date.now();
for (const feature of ['manual', 'typed', 'bon']) {
  const target = `target/measure-${stamp}-${feature}`;
  const started = performance.now();
  pass(['cargo', 'check', '--locked', '--no-default-features', '--features', feature, '--target-dir', target]);
  const fresh = performance.now() - started;
  const warmStarted = performance.now();
  pass(['cargo', 'check', '--locked', '--no-default-features', '--features', feature, '--target-dir', target]);
  observations.push(`OBSERVE ${feature}: fresh-target check ${fresh.toFixed(0)} ms; incremental check ${(performance.now() - warmStarted).toFixed(0)} ms; target files ${bytes(target)} bytes\n`);
}
writeFileSync('evidence/observations.txt', observations.join(''));
process.stdout.write(observations.join(''));
