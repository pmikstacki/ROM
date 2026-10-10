import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, symlinkSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { assertPnpmVersions } from './pnpm-verification.mjs';
function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'rom-pnpm-versions-'));
  const put = (path, name, version) => { mkdirSync(join(root,path),{recursive:true}); writeFileSync(join(root,path,'package.json'),JSON.stringify({name,version})); };
  return {root,put,cleanup:()=>rmSync(root,{recursive:true,force:true})};
}
const expected = { packages: { 'node_modules/a': {version:'1.0.0'}, 'node_modules/a/node_modules/b': {version:'2.0.0'}, 'node_modules/b': {version:'3.0.0'} } };
test('pnpm version checks preserve exact versions when the hoisted dependency layout changes', () => {
  const f=fixture();try{f.put('node_modules/a','a','1.0.0');f.put('node_modules/b','b','2.0.0');f.put('node_modules/a/node_modules/b','b','3.0.0');assert.equal(assertPnpmVersions(f.root,expected).length,3);}finally{f.cleanup();}
});
test('pnpm version checks reject missing, unexpected and altered installed versions', () => {
  const f=fixture();try{f.put('node_modules/a','a','1.0.0');f.put('node_modules/b','b','2.0.0');assert.throws(()=>assertPnpmVersions(f.root,expected),/missing/);f.put('node_modules/a/node_modules/b','b','3.0.1');assert.throws(()=>assertPnpmVersions(f.root,expected),/unexpected|missing/);}finally{f.cleanup();}
});
test('pnpm version checks reject a dependency linked outside the consumer installation', () => {
  const f=fixture(),other=fixture();try{other.put('external','a','1.0.0');mkdirSync(join(f.root,'node_modules'));symlinkSync(join(other.root,'external'),join(f.root,'node_modules/a'),'dir');assert.throws(()=>assertPnpmVersions(f.root,expected),/outside/);}finally{f.cleanup();other.cleanup();}
});
