import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { isolatedPodman, requireIsolatedInfo } from './environment.mjs';

test('engine configuration names every isolated store and excludes inherited remote/credential variables', () => {
  const root = mkdtempSync(join(tmpdir(), 'rom-identity-engine-'));
  const engine = isolatedPodman(root);
  assert.ok(engine.args.includes('--remote=false'));
  for (const name of ['root', 'runroot', 'volumepath', 'network-config-dir', 'tmpdir', 'hooks-dir']) {
    assert.ok(engine.args.some(arg => arg.startsWith(`--${name}=${root}/`)));
  }
  assert.equal(engine.env.CONTAINER_HOST, undefined); assert.equal(engine.env.DOCKER_HOST, undefined);
  assert.ok(engine.env.REGISTRY_AUTH_FILE.startsWith(root));
});

test('isolated INFO rejects wrong store, remote service, shared driver, and existing containers', () => {
  const root = mkdtempSync(join(tmpdir(), 'rom-identity-info-')); const engine = isolatedPodman(root);
  const info = { host: { serviceIsRemote: false }, store: { graphRoot: join(root, 'engine/root'), runRoot: join(root, 'engine/runroot'), graphDriverName: 'vfs', containerStore: { number: 0 }, imageStore: { number: 0 } } };
  assert.doesNotThrow(() => requireIsolatedInfo(engine, info));
  for (const mutate of [
    i => { i.host.serviceIsRemote = true; }, i => { i.store.graphRoot = '/var/lib/containers/storage'; },
    i => { i.store.runRoot = '/run/containers/storage'; }, i => { i.store.graphDriverName = 'overlay'; },
    i => { i.store.containerStore.number = 1; },
  ]) { const changed = structuredClone(info); mutate(changed); assert.throws(() => requireIsolatedInfo(engine, changed), /isolated engine/); }
});

test('bounded engine configuration places all storage under admitted acquisition filesystem', () => {
 const root=mkdtempSync(join(tmpdir(),'rom-identity-bounded-'));
 const engine=isolatedPodman(root, { directory:'bounded' });
 assert.ok(engine.args.includes(`--root=${root}/bounded/root`));
 assert.equal(engine.env.TMPDIR,`${root}/bounded/download`);
 const info={host:{serviceIsRemote:false},store:{graphRoot:`${root}/bounded/root`,runRoot:`${root}/bounded/runroot`,graphDriverName:'vfs',containerStore:{number:0},imageStore:{number:0}}};
 assert.doesNotThrow(()=>requireIsolatedInfo(engine,info));
 assert.throws(()=>isolatedPodman(root,{directory:'../escape'}),/engine directory/);
});

test('explicit overlay lane records its driver and rejects shared or unknown drivers', () => {
 const root=mkdtempSync(join(tmpdir(),'rom-identity-overlay-'));const engine=isolatedPodman(root,{driver:'overlay'});
 assert.ok(engine.args.includes('--storage-driver=overlay'));
 const info={host:{serviceIsRemote:false},store:{graphRoot:`${root}/engine/root`,runRoot:`${root}/engine/runroot`,graphDriverName:'overlay',containerStore:{number:0},imageStore:{number:0}}};
 assert.doesNotThrow(()=>requireIsolatedInfo(engine,info));
 assert.throws(()=>isolatedPodman(root,{driver:'btrfs'}),/engine driver/);
});
