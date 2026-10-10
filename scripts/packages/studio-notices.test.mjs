import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync, readFileSync, rmSync, renameSync, symlinkSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { collectRuntimeNotices } from '../../studio/build/notices/collection.mjs';
import { validateRuntimeNotices } from '../../studio/build/notices/inventory.mjs';
import { studioAssets } from './studio-assets.mjs';
import { assetFixture } from './studio-test-support.mjs';

function fixture() {
  const workspace = mkdtempSync(join(tmpdir(), 'rom-runtime-notice-test-'));
  const root = join(workspace, 'studio');
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
  return { root, put, bundle, license, cleanup: () => rmSync(workspace, { recursive: true, force: true }) };
}

test('shared installation retains portable module IDs and exact licenses', () => {
  const f = fixture(), shared = mkdtempSync(join(tmpdir(), 'rom-shared-notices-'));
  try {
    renameSync(join(f.root, 'node_modules'), join(shared, 'node_modules'));
    symlinkSync(join(shared, 'node_modules'), join(f.root, 'node_modules'), 'dir');
    const emitted = f.bundle['assets/main.js'].modules;
    delete emitted[join(f.root, 'node_modules/svelte-toolbelt/index.js')];
    emitted[join(shared, 'node_modules/svelte-toolbelt/index.js') + '?transformed'] = { renderedLength: 11 };
    const assets = collectRuntimeNotices(f.root, f.bundle);
    const inventory = JSON.parse(assets.get('third-party-notices.json'));
    const owner = inventory.owners.find(value => value.name === 'svelte-toolbelt');
    assert.ok(owner, 'the shared installation must retain its runtime owner');
    assert.deepEqual(owner.modules, ['node_modules/svelte-toolbelt/index.js?transformed']);
    assert.deepEqual(assets.get(owner.notices.find(value => value.source === 'LICENSE').path), f.license);
    assert.equal(JSON.stringify(inventory).includes(shared), false);
    validateRuntimeNotices(new Map([...assets,
      ['assets/main.js', Buffer.from(f.bundle['assets/main.js'].code)],
      ['assets/main.css', Buffer.from(f.bundle['assets/main.css'].source)]]));
  } finally { f.cleanup(); rmSync(shared, { recursive: true, force: true }); }
});

test('shared installation still rejects missing licenses, changed identity and external modules', () => {
  for (const mutation of ['missing-license', 'package-name', 'external']) {
    const f = fixture(), shared = mkdtempSync(join(tmpdir(), 'rom-shared-notices-'));
    try {
      renameSync(join(f.root, 'node_modules'), join(shared, 'node_modules'));
      symlinkSync(join(shared, 'node_modules'), join(f.root, 'node_modules'), 'dir');
      if (mutation === 'missing-license') rmSync(join(shared, 'node_modules/svelte-toolbelt/LICENSE'));
      if (mutation === 'package-name') writeFileSync(join(shared, 'node_modules/svelte-toolbelt/package.json'), JSON.stringify({ name: 'wrong-owner', version: '1.2.3' }));
      if (mutation === 'external') f.bundle['assets/main.js'].modules[join(shared, 'outside-installation.js')] = { renderedLength: 1 };
      assert.throws(() => collectRuntimeNotices(f.root, f.bundle), mutation === 'missing-license'
        ? /missing runtime notice license text/ : mutation === 'package-name'
          ? /invalid runtime notice package identity/ : /unclassified runtime notice external module/);
    } finally { f.cleanup(); rmSync(shared, { recursive: true, force: true }); }
  }
});

test('package links cannot extend the configured installation boundary', () => {
  for (const physical of [false, true]) {
    const f = fixture(), external = mkdtempSync(join(tmpdir(), 'rom-external-package-'));
    try {
      writeFileSync(join(external, 'package.json'), JSON.stringify({ name: 'svelte-toolbelt', version: '1.2.3', license: 'MIT' }));
      writeFileSync(join(external, 'LICENSE'), f.license);
      writeFileSync(join(external, 'index.js'), 'export const fixture = 1;');
      rmSync(join(f.root, 'node_modules/svelte-toolbelt'), { recursive: true });
      symlinkSync(external, join(f.root, 'node_modules/svelte-toolbelt'), 'dir');
      const modules = f.bundle['assets/main.js'].modules;
      delete modules[join(f.root, 'node_modules/svelte-toolbelt/index.js')];
      modules[physical ? join(external, 'index.js') : join(f.root, 'node_modules/svelte-toolbelt/index.js')] = { renderedLength: 11 };
      assert.throws(() => collectRuntimeNotices(f.root, f.bundle), /unclassified runtime notice (package|external module)/);
    } finally { f.cleanup(); rmSync(external, { recursive: true, force: true }); }
  }
});

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

function svarFixture() {
  const f = fixture();
  const vendor = 'src/lib/filters/vendor';
  const license = readFileSync(new URL('../../studio/src/lib/filters/vendor/LICENSE', import.meta.url));
  f.put(`${vendor}/LICENSE`, license);
  f.put(`${vendor}/provenance.json`, readFileSync(new URL('../../studio/src/lib/filters/vendor/provenance.json', import.meta.url)));
  for (const path of ['Rule.svelte', 'composition.ts']) {
    f.put(`${vendor}/${path}`, 'fixture runtime');
    f.bundle['assets/main.js'].modules[join(f.root, `${vendor}/${path}`)] = { renderedLength: 1 };
  }
  f.bundle['assets/main.js'].modules[join(f.root, `${vendor}/Rule.svelte`) + '?transformed'] = { renderedLength: 1 };
  return { ...f, svarLicense: license };
}

function emittedFiles(f, assets) {
  return new Map([...assets, ['assets/main.js', Buffer.from(f.bundle['assets/main.js'].code)],
    ['assets/main.css', Buffer.from(f.bundle['assets/main.css'].source)]]);
}

test('emitted SVAR modules retain the original MIT notice with their pinned source commit', () => {
  const f = svarFixture();
  try {
    const assets = collectRuntimeNotices(f.root, f.bundle);
    const inventory = validateRuntimeNotices(emittedFiles(f, assets));
    const owner = inventory.owners.find(owner => owner.id === 'vendored:svar-filter');
    assert.ok(owner, 'missing SVAR runtime owner');
    assert.equal(owner.kind, 'vendored');
    assert.equal(owner.name, 'svar-filter-vendored');
    assert.equal(owner.version, '1c581c3312c626c525ee64b8f94446a025fa141c');
    assert.equal(owner.license_expression, 'MIT');
    assert.deepEqual(owner.modules, ['src/lib/filters/vendor/Rule.svelte', 'src/lib/filters/vendor/Rule.svelte?transformed', 'src/lib/filters/vendor/composition.ts']);
    assert.deepEqual(owner.notices.map(notice => notice.source), ['LICENSE']);
    assert.deepEqual(assets.get(owner.notices[0].path), f.svarLicense);
    assert.equal(owner.notices[0].sha256, '873d0542c84ec8a7ecaf127c18ae1209ab319bc5f5a2efbb6160da8956c2787f');
  } finally { f.cleanup(); }
});

test('inventory rejects emitted SVAR code stripped of its required owner', () => {
  const f = svarFixture();
  try {
    const assets = collectRuntimeNotices(f.root, f.bundle);
    const inventory = JSON.parse(assets.get('third-party-notices.json'));
    for (const owner of inventory.owners.filter(owner => owner.id === 'vendored:svar-filter'))
      for (const notice of owner.notices) assets.delete(notice.path);
    inventory.owners = inventory.owners.filter(owner => owner.id !== 'vendored:svar-filter');
    for (const output of inventory.outputs)
      for (const module of output.modules) if (module.id.startsWith('src/lib/filters/vendor/')) module.owners = [];
    assets.set('third-party-notices.json', Buffer.from(JSON.stringify(inventory)));
    assert.throws(() => validateRuntimeNotices(emittedFiles(f, assets)), /missing runtime notice module owner/);
  } finally { f.cleanup(); }
});

test('SVAR owner cannot silently change its source commit or retained license profile', () => {
  for (const mutation of ['provenance', 'license']) {
    const f = svarFixture();
    try {
      if (mutation === 'provenance') {
        const path = 'src/lib/filters/vendor/provenance.json';
        const provenance = JSON.parse(readFileSync(join(f.root, path)));
        provenance.commit = '0'.repeat(40); f.put(path, JSON.stringify(provenance));
      } else f.put('src/lib/filters/vendor/LICENSE', 'MIT replacement text');
      assert.throws(() => collectRuntimeNotices(f.root, f.bundle), /runtime notice/);
    } finally { f.cleanup(); }
  }
});


test('runtime polyfill ownership resolves rolldown through the installed Vite dependency tree', () => {
  const f = fixture();
  try {
    const virtual = join(f.root, 'node_modules/.pnpm/rolldown@1.2.3/node_modules');
    mkdirSync(virtual, { recursive: true });
    renameSync(join(f.root, 'node_modules/rolldown'), join(virtual, 'rolldown'));
    mkdirSync(join(f.root, 'node_modules/vite/node_modules'), { recursive: true });
    symlinkSync(join(virtual, 'rolldown'), join(f.root, 'node_modules/vite/node_modules/rolldown'), 'dir');
    const inventory = JSON.parse(collectRuntimeNotices(f.root, f.bundle).get('third-party-notices.json'));
    assert.ok(inventory.owners.some(owner => owner.id === 'tool-runtime:rolldown@1.2.3'));
  } finally { f.cleanup(); }
});

test('ROM UI styles retain their nested shadcn and animation owners', () => {
  const f = fixture();
  try {
    f.put('node_modules/rom-ui/package.json', JSON.stringify({ name: 'rom-ui', version: '1.2.3', license: 'MIT', exports: { './styles': './src/styles.css' } }));
    f.put('node_modules/rom-ui/LICENSE', f.license);
    f.put('node_modules/rom-ui/src/styles.css', '@import "shadcn-svelte/tailwind.css";\n@import "tw-animate-css";');
    for (const name of ['shadcn-svelte', 'tw-animate-css']) {
      const directory = 'node_modules/rom-ui/node_modules/' + name;
      f.put(directory + '/package.json', JSON.stringify({ name, version: '1.2.3', license: 'MIT', exports: name === 'shadcn-svelte' ? { './tailwind.css': './dist/style.css' } : './dist/style.css' }));
      f.put(directory + '/LICENSE', f.license);
      f.put(directory + '/dist/style.css', '/* nested CSS */ .fixture {}');
    }
    f.put('src/app.css', '@import "rom-ui/styles";');
    const inventory = JSON.parse(collectRuntimeNotices(f.root, f.bundle).get('third-party-notices.json'));
    for (const name of ['rom-ui', 'shadcn-svelte', 'tw-animate-css'])
      assert.ok(inventory.owners.some(owner => owner.name === name && owner.modules.some(id => id.startsWith('generated-css:') && id.includes('sha256='))), name);
  } finally { f.cleanup(); }
});

test('shared installation retains portable imported stylesheet IDs', () => {
  const f = fixture(), shared = mkdtempSync(join(tmpdir(), 'rom-shared-css-'));
  try {
    f.put('node_modules/tw-animate-css/package.json', JSON.stringify({ name: 'tw-animate-css', version: '1.2.3', license: 'MIT', exports: './style.css' }));
    f.put('node_modules/tw-animate-css/LICENSE', f.license);
    f.put('node_modules/tw-animate-css/style.css', '.fixture {}');
    f.put('src/app.css', '@import "tw-animate-css";');
    renameSync(join(f.root, 'node_modules'), join(shared, 'node_modules'));
    symlinkSync(join(shared, 'node_modules'), join(f.root, 'node_modules'), 'dir');
    const inventory = JSON.parse(collectRuntimeNotices(f.root, f.bundle).get('third-party-notices.json'));
    const owner = inventory.owners.find(owner => owner.name === 'tw-animate-css');
    assert.ok(owner.modules.some(id => id.startsWith('generated-css:node_modules/tw-animate-css/style.css#sha256=')));
    assert.equal(JSON.stringify(inventory).includes(shared), false);
  } finally { f.cleanup(); rmSync(shared, { recursive: true, force: true }); }
});
