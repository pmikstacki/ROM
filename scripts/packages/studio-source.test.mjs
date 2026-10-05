import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync, symlinkSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { studioSource, copyStudio, isGeneratedStudioPath } from './studio-source.mjs';
import { frontendFixture as fixture } from './studio-test-support.mjs';

test('Studio copied inputs retain complete regular sources and exact npm identities', () => {
  const f = fixture();
  try {
    for (const path of ['node_modules/cache', 'dist/index.html', '.component-dist/page.html', '.demo-dist/index.html', '.demo-field-dist/index.html', 'test-results/log', 'playwright-report/index.html']) f.put(path, 'generated');
    const before = studioSource(f.root);
    assert.equal(before.package_version, '0.0.2');
    assert.ok(Object.hasOwn(before.files, '.gitignore'));
    assert.ok(Object.hasOwn(before.files, 'tests/input.ts'));
    const copied = copyStudio(f.root, join(f.root, 'external'));
    assert.deepEqual(studioSource(join(f.root, 'external')), before);
    assert.equal(readFileSync(join(copied, 'src/App.svelte'), 'utf8'), 'source');
    for (const name of ['node_modules', 'dist', '.component-dist', '.demo-dist', '.demo-field-dist', 'test-results', 'playwright-report']) assert.equal(existsSync(join(copied, name)), false);
    assert.throws(() => copyStudio(f.root, join(f.root, 'external')), /exist/);
    f.put('src/App.svelte', 'changed');
    assert.notEqual(studioSource(f.root).sha256, before.sha256);
  } finally { rmSync(f.root, { recursive: true, force: true }); }
});

test('Studio source refuses incomplete inputs, incompatible npm lock, and source links', () => {
  for (const mutation of ['missing', 'version', 'dependency', 'link']) {
    const f = fixture();
    try {
      if (mutation === 'missing') rmSync(join(f.root, 'studio/index.html'));
      else if (mutation === 'link') symlinkSync('/tmp', join(f.root, 'studio/src/link'));
      else {
        const lock = JSON.parse(readFileSync(join(f.root, 'studio/package-lock.json')));
        if (mutation === 'version') lock.packages[''].version = '9.9.9';
        else lock.packages[''].dependencies.svelte = 'unapproved';
        f.put('package-lock.json', JSON.stringify(lock));
      }
      assert.throws(() => studioSource(f.root), /Studio|regular|symbolic/);
    } finally { rmSync(f.root, { recursive: true, force: true }); }
  }
  assert.equal(isGeneratedStudioPath('dist/index.html'), true);
  assert.equal(isGeneratedStudioPath('src/dist/example.ts'), false);
});

test('private frontend environment and key inputs are never admitted to a fresh copy', () => {
  for (const path of ['.env', '.env.production', 'src/private.key', 'tests/private.pem']) {
    const f = fixture();
    try {
      f.put(path, 'private fixture marker');
      assert.throws(() => copyStudio(f.root, join(f.root, 'external')), /private Studio input/);
      assert.equal(existsSync(join(f.root, 'external')), false);
    } finally { rmSync(f.root, { recursive: true, force: true }); }
  }
});
