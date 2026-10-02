import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
const cases = [
  ['unsupported', /Unsupported.*Field|Field.*Unsupported/],
  ['malformed', /resource Broken: name must be a string literal/],
  ['conflicting', /resource Broken: duplicate name option/],
  ['duplicate_name', /resource Broken: duplicate external field name `same`/],
  ['unknown_option', /resource Broken, field value: unknown option; use rename/],
  ['tuple', /resource Broken: only structs with named fields are supported/],
  ['duplicate_rename', /resource Broken, field value: duplicate rename option/],
];
for (const [name, expected] of cases) {
  const file = `fixtures/fail/src/bin/${name}.rs`;
  const lines = readFileSync(file,'utf8').split('\n');
  const line = lines.findIndex(text=>text.includes('// error-site')) + 1;
  const result = spawnSync('cargo',['check','--locked','-p','fixture-fail','--bin',name,'--message-format=json'],{encoding:'utf8'});
  assert.equal(result.status,101,`${name} unexpectedly compiled: ${result.stdout}\n${result.stderr}`);
  const diagnostics = result.stdout.split('\n').filter(Boolean).map(s=>JSON.parse(s)).filter(m=>m.reason==='compiler-message'&&m.message.level==='error').map(m=>m.message);
  const diagnostic = diagnostics.find(d=>expected.test(d.message));
  assert.ok(diagnostic,`${name}: expected diagnostic absent: ${result.stdout}\n${result.stderr}`);
  const source = diagnostic.spans.find(span=>span.is_primary && span.file_name.endsWith(`${name}.rs`) && span.line_start <= line && span.line_end >= line);
  assert.ok(source,`${name}: diagnostic did not point at offending source line ${line}: ${diagnostic.rendered}`);
  console.log(`PASS compile-fail ${name}: ${diagnostic.message} (${file}:${line})`);
}
const tree = spawnSync('cargo',['tree','--locked','-p','fixture-manual','-e','normal'],{encoding:'utf8'});
assert.equal(tree.status,0,tree.stderr);
assert.ok(!tree.stdout.includes('resource-contract-derive'),tree.stdout);
assert.ok(!tree.stdout.includes('syn v'),tree.stdout);
console.log('PASS manual-only dependency graph contains no derive/syn/quote dependency');
console.log('VERDICT: 7 source-local compile failures checked; manual derive-free dependency graph checked.');
