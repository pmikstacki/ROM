import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,cpSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fixtureSources,copiedSources,requireCopiedInputs,requireFixtureFreeze} from './fixture-fence.mjs';
function fixture(){const base=mkdtempSync(join(tmpdir(),'rom-ai-fixture-fence-')),maintained=join(base,'maintained'),copied=join(base,'copied');mkdirSync(join(maintained,'consumer/src'),{recursive:true});writeFileSync(join(maintained,'consumer/src/App.svelte'),'authorized composition');writeFileSync(join(maintained,'proxy.mjs'),'real transport');writeFileSync(join(maintained,'server.mjs'),'entry');writeFileSync(join(maintained,'native-process.mjs'),'child');writeFileSync(join(maintained,'protocol.mjs'),'contract');writeFileSync(join(maintained,'failure-observation.mjs'),'observations');mkdirSync(copied);cpSync(join(maintained,'consumer'),join(copied,'consumer'),{recursive:true});mkdirSync(join(copied,'runtime'));for(const file of ['proxy.mjs','server.mjs','native-process.mjs','protocol.mjs','failure-observation.mjs'])cpSync(join(maintained,file),join(copied,'runtime',file));return {maintained,copied};}
test('copy fence binds Svelte and native transport inputs and detects either mutation',()=>{
  const {maintained,copied}=fixture(),expected=fixtureSources(maintained);requireCopiedInputs(expected,copiedSources(copied));writeFileSync(join(copied,'consumer/src/App.svelte'),'changed after build');assert.throws(()=>requireCopiedInputs(expected,copiedSources(copied)),/copied fixture/);
  const other=fixture(),selected=fixtureSources(other.maintained);writeFileSync(join(other.copied,'runtime/proxy.mjs'),'changed after browser');assert.throws(()=>requireCopiedInputs(selected,copiedSources(other.copied)),/copied fixture/);
});
test('only explicit generated outputs are excluded; added input files fail the copy fence',()=>{
  const {maintained,copied}=fixture(),expected=fixtureSources(maintained);
  for(const directory of ['node_modules','dist','test-results','playwright-report']){mkdirSync(join(copied,'consumer',directory));writeFileSync(join(copied,'consumer',directory,'generated'),'generated');}
  requireCopiedInputs(expected,copiedSources(copied));writeFileSync(join(copied,'consumer/src/added.ts'),'unselected input');assert.throws(()=>requireCopiedInputs(expected,copiedSources(copied)),/copied fixture/);
});
test('independent selected freeze rejects missing, duplicated or changed authoritative inputs',()=>{
  const {maintained}=fixture(),current=fixtureSources(maintained);
  const selected={files:Object.entries(current).map(([path,sha256])=>({path:'studio/tests/ai-flow-progress/'+path,sha256}))};
  assert.deepEqual(requireFixtureFreeze(selected,current),current);
  assert.throws(()=>requireFixtureFreeze({files:selected.files.slice(1)},current),/selected freeze/);
  assert.throws(()=>requireFixtureFreeze({files:[...selected.files,selected.files[0]]},current),/invalid fixture freeze/);
  writeFileSync(join(maintained,'consumer/src/App.svelte'),'changed authoritative view');
  assert.throws(()=>requireFixtureFreeze(selected,fixtureSources(maintained)),/selected freeze/);
});
