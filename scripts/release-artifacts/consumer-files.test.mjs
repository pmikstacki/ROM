import test from 'node:test';
import assert from 'node:assert/strict';
import { writeFileSync, rmSync, symlinkSync, linkSync } from 'node:fs';
import { join } from 'node:path';
import { packageFixture, archiveFixture } from './consumer-fixture.mjs';
import { hash } from '../skills/files.mjs';
import * as evidence from './consumer-files.mjs';

const fixture = packageFixture;

test('retained package bytes and lock are checked against the source lease for all three consumers', () => {
  for (const kind of ['controls', 'recovery-sqlite', 'recovery-redb']) {
    const f = fixture(kind);
    assert.equal(evidence.requireConsumerFiles(f.lease, f.directory, kind, f.record), f.record);
    assert.equal(f.fences(), 2);
  }
});

test('rewritten report hashes cannot hide changed installed source or consumer lock', () => {
  for (const changed of ['src/index.ts', 'LICENSE', 'lock', 'extra']) {
    const f = fixture();
    if (changed === 'lock') {
      writeFileSync(join(f.directory, 'consumer/package-lock.json'), 'changed');
      f.record.consumer_lock_sha256 = hash(join(f.directory, 'consumer/package-lock.json'));
    } else {
      const path = changed === 'extra' ? 'src/extra.ts' : changed;
      writeFileSync(join(f.installed, path), 'changed');
      f.record.source_files[path] = hash(join(f.installed, path));
    }
    assert.throws(() => evidence.requireConsumerFiles(f.lease, f.directory, f.kind, f.record), /consumer file evidence/);
  }
});

test('changed archive bytes, symlinks and hardlinks fail retained file admission', () => {
  for (const changed of ['archive', 'symlink', 'hardlink']) {
    const f = fixture();
    if (changed === 'archive') writeFileSync(join(f.directory, 'studio-source.tar.gz'), 'changed archive');
    else {
      const path = join(f.installed, 'src/index.ts'); rmSync(path);
      if (changed === 'symlink') symlinkSync(join(f.lease.root, 'studio/src/index.ts'), path);
      else linkSync(join(f.lease.root, 'studio/src/index.ts'), path);
    }
    assert.throws(() => evidence.requireConsumerFiles(f.lease, f.directory, f.kind, f.record), /consumer file evidence/);
  }
});

test('the source lease is checked again after reading evidence', () => {
  const f = fixture(); let calls = 0;
  f.lease.fence = () => { if (++calls === 2) throw Error('source drift'); };
  assert.throws(() => evidence.requireConsumerFiles(f.lease, f.directory, f.kind, f.record), /consumer file evidence/);
  assert.equal(calls, 2);
});

test('a rehashed archive with changed source or source lock cannot satisfy file admission', () => {
  for (const changed of ['src/index.ts', 'package-lock.json']) {
    const f = fixture();
    writeFileSync(join(f.directory, 'source-stage/rom-studio-source', changed), 'different archived content');
    archiveFixture(f.directory);
    f.record.archive_sha256 = hash(join(f.directory, 'studio-source.tar.gz'));
    assert.throws(() => evidence.requireConsumerFiles(f.lease, f.directory, f.kind, f.record), /consumer file evidence/);
  }
});

test('retained paths require exact manifest execution context and matching extraction witness', () => {
  const f=fixture();
  const context={extracted_root:'/original/verified',evidence_root:'/original/evidence',host_binary:'/original/native/host',revision:'a'.repeat(40),tree:'b'.repeat(40),source_inventory_sha256:'c'.repeat(64),studio_sha256:'d'.repeat(64),archive_sha256:'e'.repeat(64)};
  f.lease.witness=Object.fromEntries(['revision','tree','source_inventory_sha256','studio_sha256','archive_sha256'].map(key=>[key,context[key]]));
  f.record.source_input='/original/verified/studio';f.record.installed_package='/original/evidence/controls/consumer/node_modules/rom-studio';
  assert.equal(evidence.requireConsumerFiles(f.lease,f.directory,f.kind,f.record,context),f.record);
  assert.throws(()=>evidence.requireConsumerFiles(f.lease,f.directory,f.kind,f.record,{...context,archive_sha256:'f'.repeat(64)}),/consumer file evidence/);
  f.record.installed_package='/unrelated/package';assert.throws(()=>evidence.requireConsumerFiles(f.lease,f.directory,f.kind,f.record,context),/consumer file evidence/);
});
