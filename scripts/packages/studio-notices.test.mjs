import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { collectRuntimeNotices } from '../../studio/build/notices/collection.mjs';
import { validateRuntimeNotices } from '../../studio/build/notices/inventory.mjs';
import { studioAssets } from './studio-assets.mjs';
import { assetFixture } from './studio-test-support.mjs';

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'rom-runtime-notice-test-'));
  const put = (path, content) => { mkdirSync(dirname(join(root, path)), { recursive: true }); writeFileSync(join(root, path), content); };
  const license = Buffer.from('MIT License\r\nCopyright fixture\r\n\xc2\xa9\n', 'utf8');
  for (const name of ['svelte-toolbelt', 'vite', 'rolldown', 'tailwindcss', 'compiler-only']) {
    put(`node_modules/${name}/package.json`, JSON.stringify({ name, version: '1.2.3', ...(name === 'svelte-toolbelt' ? {} : { license: 'MIT' }) }));
    put(`node_modules/${name}/LICENSE`, license);
    put(`node_modules/${name}/index.js`, 'export const fixture = 1;');
  }
  put('src/lib/components/ui/LICENSE.md', license);
  put('src/lib/components/ui/button/button.svelte', '<button/>');
  const bundle = { 'assets/main.js': { type: 'chunk', fileName: 'assets/main.js', code: 'console.log("fixture");',
    modules: { [join(root, 'node_modules/svelte-toolbelt/index.js')]: { renderedLength: 11 },
      [join(root, 'node_modules/compiler-only/index.js')]: { renderedLength: 0 },
      [join(root, 'src/lib/components/ui/button/button.svelte')]: { renderedLength: 9 },
      '\0vite/modulepreload-polyfill.js': { renderedLength: 21 } } },
    'assets/main.css': { type: 'asset', fileName: 'assets/main.css', source: '/*! tailwindcss v1.2.3 | MIT License */body{}' } };
  return { root, put, bundle, license, cleanup: () => rmSync(root, { recursive: true, force: true }) };
}

test('emitted runtime ownership retains exact license and notice bytes without requiring SPDX', () => {
  const f = fixture();
  try {
    f.put('node_modules/svelte-toolbelt/NOTICE.txt', 'Unmodified upstream notice.\n');
    f.put('node_modules/svelte-toolbelt/dist/icons/copyright.js', 'export const copyrightIcon = "icon";');
    const result = collectRuntimeNotices(f.root, f.bundle);
    const inventory = JSON.parse(result.get('third-party-notices.json'));
    assert.deepEqual(inventory.owners.map(x => x.name), ['rolldown', 'shadcn-svelte-vendored', 'svelte-toolbelt', 'tailwindcss', 'vite']);
    const toolbelt = inventory.owners.find(x => x.name === 'svelte-toolbelt');
    assert.equal(toolbelt.license_expression, null);
    assert.deepEqual(result.get(toolbelt.notices.find(x => x.source === 'LICENSE').path), f.license);
    assert.equal(result.get(toolbelt.notices.find(x => x.source === 'NOTICE.txt').path).toString(), 'Unmodified upstream notice.\n');
    assert.equal(inventory.outputs[0].modules.some(x => x.id.includes('compiler-only')), false);
    const files = new Map([...result, ['assets/main.js', Buffer.from(f.bundle['assets/main.js'].code)],
      ['assets/main.css', Buffer.from(f.bundle['assets/main.css'].source)]]);
    validateRuntimeNotices(files);
  } finally { f.cleanup(); }
});

test('build refuses missing full license text, unknown virtual code, or an untrusted package owner', () => {
  for (const mutation of ['missing-license', 'virtual', 'package-name', 'outside']) {
    const f = fixture();
    try {
      if (mutation === 'missing-license') rmSync(join(f.root, 'node_modules/svelte-toolbelt/LICENSE'));
      else if (mutation === 'virtual') f.bundle['assets/main.js'].modules['\0unclassified/plugin-runtime'] = { renderedLength: 1 };
      else if (mutation === 'package-name') f.put('node_modules/svelte-toolbelt/package.json', JSON.stringify({ name: '../escape', version: '1' }));
      else f.bundle['assets/main.js'].modules['/unclassified/external-runtime.js'] = { renderedLength: 1 };
      assert.throws(() => collectRuntimeNotices(f.root, f.bundle), /runtime notice/);
    } finally { f.cleanup(); }
  }
});

test('asset admission fails closed on missing inventory, missing or changed notices, and uncovered JavaScript', () => {
  for (const mutation of ['inventory', 'license', 'changed', 'uncovered']) {
    const root = mkdtempSync(join(tmpdir(), 'rom-notice-admission-test-'));
    try {
      assetFixture(root);
      if (mutation === 'inventory') rmSync(join(root, 'third-party-notices.json'), { force: true });
      else {
        const path = 'notices/fixture/LICENSE';
        if (mutation === 'license') rmSync(join(root, path), { force: true });
        else if (mutation === 'changed') writeFileSync(join(root, path), 'changed');
        else writeFileSync(join(root, 'assets/extra.js'), 'uncovered runtime');
      }
      assert.throws(() => studioAssets(root), /runtime notice/);
    } finally { rmSync(root, { recursive: true, force: true }); }
  }
});

test('inventory rejects missing owner bindings and malformed notice metadata before admission', () => {
  for (const mutation of ['owner', 'hash', 'array', 'unlisted', 'notice-only']) {
    const root = mkdtempSync(join(tmpdir(), 'rom-notice-metadata-test-'));
    try {
      assetFixture(root);
      const path = join(root, 'third-party-notices.json');
      const inventory = JSON.parse(readFileSync(path));
      if (mutation === 'owner') inventory.outputs[0].modules[0].owners = [];
      else if (mutation === 'hash') inventory.owners[0].notices[0].sha256 = 'invalid';
      else if (mutation === 'array') { writeFileSync(path, '[]'); assert.throws(() => studioAssets(root), /runtime notice/); continue; }
      else if (mutation === 'unlisted') writeFileSync(join(root, 'notices/extra.txt'), 'extra notice');
      else inventory.owners[0].notices[0].source = 'NOTICE';
      writeFileSync(path, JSON.stringify(inventory));
      assert.throws(() => studioAssets(root), /runtime notice/);
    } finally { rmSync(root, { recursive: true, force: true }); }
  }
});

test('distinct transformed module IDs preserve query suffixes in the emitted inventory', () => {
  const f = fixture();
  try {
    f.bundle['assets/main.js'].modules[join(f.root, 'node_modules/svelte-toolbelt/index.js') + '?transformed'] = { renderedLength: 1 };
    const assets = collectRuntimeNotices(f.root, f.bundle);
    const inventory = JSON.parse(assets.get('third-party-notices.json'));
    assert.deepEqual(inventory.owners.find(x => x.name === 'svelte-toolbelt').modules,
      ['node_modules/svelte-toolbelt/index.js', 'node_modules/svelte-toolbelt/index.js?transformed']);
  } finally { f.cleanup(); }
});

test('unclassified JavaScript assets cannot bypass emitted runtime provenance', () => {
  const f = fixture();
  try {
    f.bundle['assets/unclassified.js'] = { type: 'asset', fileName: 'assets/unclassified.js', source: 'thirdParty=true;' };
    assert.throws(() => collectRuntimeNotices(f.root, f.bundle), /runtime notice JavaScript provenance/);
  } finally { f.cleanup(); }
});

test('copied responsive hook and imported CSS retain their actual owners', () => {
  const f = fixture();
  try {
    f.put('src/lib/hooks/is-mobile.svelte.ts', 'export const mobile = true;');
    f.bundle['assets/main.js'].modules[join(f.root, 'src/lib/hooks/is-mobile.svelte.ts')] = { renderedLength: 1 };
    for (const name of ['shadcn-svelte', 'tw-animate-css']) {
      f.put(`node_modules/${name}/package.json`, JSON.stringify({ name, version: '1.2.3', license: 'MIT', exports: name === 'shadcn-svelte' ? { './tailwind.css': { style: './dist/style.css' } } : './dist/style.css' }));
      f.put(`node_modules/${name}/LICENSE`, f.license);
      f.put(`node_modules/${name}/dist/style.css`, '/* original CSS */ .fixture { display: block; }');
    }
    f.put('src/app.css', '@import "shadcn-svelte/tailwind.css";\n@import "tw-animate-css";');
    const inventory = JSON.parse(collectRuntimeNotices(f.root, f.bundle).get('third-party-notices.json'));
    assert.ok(inventory.owners.find(owner => owner.name === 'shadcn-svelte-vendored').modules.includes('src/lib/hooks/is-mobile.svelte.ts'));
    for (const name of ['shadcn-svelte', 'tw-animate-css'])
      assert.ok(inventory.owners.some(owner => owner.name === name && owner.modules.some(id => id.startsWith('generated-css:') && id.includes('sha256='))));
    f.put('node_modules/tw-animate-css/LICENSE', '');
    assert.throws(() => collectRuntimeNotices(f.root, f.bundle), /runtime notice/);
  } finally { f.cleanup(); }
});
