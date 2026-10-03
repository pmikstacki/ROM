import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, rmSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { copyApplication, auditPackagePaths } from './application.mjs';

test('application copy keeps required test support but excludes private files and build caches', () => {
  const dir = mkdtempSync(join(tmpdir(), 'rom-package-app-test-'));
  const root = join(dir, 'source');
  const put = (path, content = 'fixture') => { const full = join(root, path); mkdirSync(join(full, '..'), {recursive:true}); writeFileSync(full, content); };
  try {
    for (const path of ['demo/settings.toml', 'demo/src/main.rs', 'demo/tests/example.rs', 'demo/README.md', 'demo/provider-fixture/processes.mjs', 'demo/provider-fixture/package-lock.json', 'demo/provider-fixture/node_modules/private/key', 'demo/.env', 'demo/target/data', 'tests/persistence/tests/support/child_process.rs', 'tests/persistence/private-auth', 'Cargo.lock']) put(path);
    const pkg = {name:'rom-demo', version:'0.1.0-alpha.1', edition:'2024', manifest_path:join(root,'demo/Cargo.toml'), dependencies:[], features:{default:[]}};
    const app = copyApplication(root, pkg, join(dir,'external'), '[patch.crates-io]\n', {sharedSupport:true,providerFixture:true});
    for (const path of ['settings.toml','src/main.rs','tests/example.rs','README.md','provider-fixture/processes.mjs','provider-fixture/package-lock.json','Cargo.lock']) assert.ok(existsSync(join(app,path)),path);
    assert.equal(readFileSync(join(dir,'external/tests/persistence/tests/support/child_process.rs'),'utf8'),'fixture');
    for (const path of ['.env','target','provider-fixture/node_modules']) assert.equal(existsSync(join(app,path)),false,path);
    assert.equal(existsSync(join(dir,'external/tests/persistence/private-auth')),false);
    assert.throws(()=>copyApplication(root,pkg,join(dir,'external'),'config',{sharedSupport:true,providerFixture:true}), /exist/);
    symlinkSync('/tmp',join(root,'demo/src/external-link'));
    assert.throws(()=>copyApplication(root,pkg,join(dir,'unsafe'),'config'), /symbolic/);
  } finally { rmSync(dir,{recursive:true,force:true}); }
});

test('dependency path audit rejects a ROM dependency resolved from the checkout or a sibling prefix', () => {
  const libraries = [{name:'rom'}, {name:'rom-http'}];
  auditPackagePaths({packages:[{name:'rom',manifest_path:'/tmp/extracted/rom/Cargo.toml'}]},libraries,'/tmp/extracted');
  for (const path of ['/source/rom/Cargo.toml','/tmp/extracted-other/rom/Cargo.toml']) assert.throws(()=>auditPackagePaths({packages:[{name:'rom',manifest_path:path}]},libraries,'/tmp/extracted'),/escaped/);
});

test('copied application inventory includes hidden configuration and shared support bytes', async () => {
  const { applicationInputs } = await import('./application.mjs');
  const dir = mkdtempSync(join(tmpdir(), 'rom-package-inventory-'));
  try {
    for (const [path, data] of [['demo/src/main.rs','source'],['demo/.cargo/config.toml','patches'],['demo/Cargo.lock','lock'],['tests/shared.rs','support']]) {
      mkdirSync(join(dir,path,'..'), {recursive:true});
      writeFileSync(join(dir,path),data);
    }
    const before = applicationInputs(dir, ['demo','tests']);
    assert.deepEqual(Object.keys(before), ['demo/.cargo/config.toml','demo/Cargo.lock','demo/src/main.rs','tests/shared.rs']);
    assert.match(before['demo/Cargo.lock'], /^[a-f0-9]{64}$/);
    writeFileSync(join(dir,'demo/src/main.rs'),'changed');
    assert.notEqual(applicationInputs(dir,['demo','tests'])['demo/src/main.rs'],before['demo/src/main.rs']);
    symlinkSync('/tmp',join(dir,'demo/link'));
    assert.throws(()=>applicationInputs(dir,['demo']),/regular/);
  } finally { rmSync(dir,{recursive:true,force:true}); }
});
