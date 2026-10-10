import test from 'node:test';import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, symlinkSync, readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';import { tmpdir } from 'node:os';
import { retainRegularEvidence } from './production-evidence.mjs';
test('retention copies selected regular bytes and excludes unrelated mutable caches',()=>{
 const parent=mkdtempSync(join(tmpdir(),'rom-retain-')),source=join(parent,'source'),target=join(parent,'retained');mkdirSync(source);mkdirSync(join(source,'reports'));writeFileSync(join(source,'reports/result.json'),'original');mkdirSync(join(source,'node_modules'));symlinkSync('/untrusted',join(source,'node_modules/alias'));
 retainRegularEvidence(source,target,['reports']);assert.equal(readFileSync(join(target,'reports/result.json'),'utf8'),'original');assert.equal(existsSync(join(target,'node_modules')),false);
});
test('retention rejects selected symlinks, path escape and occupied destination',()=>{
 const parent=mkdtempSync(join(tmpdir(),'rom-retain-negative-')),source=join(parent,'source');mkdirSync(source);symlinkSync('/untrusted',join(source,'alias'));
 assert.throws(()=>retainRegularEvidence(source,join(parent,'out'),['alias']),/symbolic asset path/);
 assert.throws(()=>retainRegularEvidence(source,join(parent,'escape'),['../source']),/asset path/);
 assert.throws(()=>retainRegularEvidence(source,join(parent,'out'),['alias']),/EEXIST/);
});

test('selected hardlinked evidence is rejected without modifying either source link',async()=>{
 const {linkSync, lstatSync}=await import('node:fs');
 const parent=mkdtempSync(join(tmpdir(),'rom-retain-hardlink-')),source=join(parent,'source');mkdirSync(source);
 writeFileSync(join(source,'binary'),'unchanged');linkSync(join(source,'binary'),join(parent,'other-link'));
 assert.throws(()=>retainRegularEvidence(source,join(parent,'retained'),['binary']),/invalid regular evidence file/);
 assert.equal(lstatSync(join(source,'binary')).nlink,2);assert.equal(readFileSync(join(parent,'other-link'),'utf8'),'unchanged');
});
