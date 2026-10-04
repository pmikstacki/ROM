import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync, rmSync, symlinkSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { studioAssets, requireStudioAssets } from './studio-assets.mjs';
import { assetFixture } from './studio-test-support.mjs';

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'rom-studio-assets-test-'));
  return assetFixture(root);
}

test('complete static asset identity rejects changed, missing, and extra files', () => {
  const root = fixture();
  try {
    const original = studioAssets(root);
    assert.deepEqual(Object.keys(original.files), ['assets/main.css', 'assets/main.js', 'index.html']);
    assert.equal(original.base_path, '/rom-studio/');
    requireStudioAssets(root, original);
    writeFileSync(join(root, 'assets/main.js'), 'changed');
    assert.throws(() => requireStudioAssets(root, original), /Studio asset identity/);
    writeFileSync(join(root, 'assets/main.js'), 'console.log("fixture");');
    writeFileSync(join(root, 'extra.txt'), 'extra');
    assert.throws(() => requireStudioAssets(root, original), /Studio asset identity/);
    rmSync(join(root, 'extra.txt'));
    rmSync(join(root, 'assets/main.js'));
    assert.throws(() => studioAssets(root), /Studio asset reference/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test('asset admission rejects links, caches, invalid UTF8, and external or escaping references', () => {
  for (const mutation of ['link', 'cache', 'utf8', 'external', 'escape', 'empty']) {
    const root = fixture();
    try {
      if (mutation === 'link') symlinkSync('/tmp', join(root, 'link'));
      else if (mutation === 'cache') { mkdirSync(join(root, 'node_modules')); writeFileSync(join(root, 'node_modules/cache'), 'cache'); }
      else if (mutation === 'utf8') writeFileSync(join(root, 'index.html'), new Uint8Array([0xc3, 0x28]));
      else if (mutation === 'external') writeFileSync(join(root, 'index.html'), '<script type="module" src="https://example.invalid/main.js"></script>');
      else if (mutation === 'escape') writeFileSync(join(root, 'index.html'), '<script type="module" src="/rom-studio/../secret.js"></script>');
      else writeFileSync(join(root, 'index.html'), '<html>no application entry</html>');
      assert.throws(() => studioAssets(root), /Studio|artifact|asset path|encoded data/);
    } finally { rmSync(root, { recursive: true, force: true }); }
  }
});
