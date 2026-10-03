import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import { assemble } from './assembly.mjs';
import { verify } from './admission.mjs';

function fixture() {
  const scratch = mkdtempSync(join(tmpdir(), 'rom-skills-test-'));
  const root = join(scratch, 'source');
  const bundle = join(scratch, 'bundle');
  const put = (path, text) => {
    mkdirSync(join(root, path, '..'), { recursive: true });
    writeFileSync(join(root, path), text);
  };
  put('Cargo.toml', '[workspace]\n');
  put('Cargo.lock', '# test source identity\n');
  put('crates/rom/src/lib.rs', 'pub const TEST: u32 = 1;\n');
  put('extensions/native-alpha-v1.json', JSON.stringify({profile_version: 1, features: ['field', 'storage', 'blob']}));
  put('docs/native-extensions.md', '# Native test profile\n');
  put('scripts/skills/verify.mjs', '// portable entrypoint\n');
  const workflows = {};
  for (const workflow of ['resource', 'native', 'operator', 'release']) {
    workflows[workflow] = { skill: `skills/rom/${workflow}/SKILL.md`, features: ['field'], asset: `skills/rom/assets/${workflow}/run.mjs`, prerequisite: 'source-checkout' };
    put(workflows[workflow].skill, `---\nname: ${workflow}\ndescription: Use when the test workflow is requested.\n---\n\n[Contract](../../../docs/native-extensions.md)\n[Example](../assets/${workflow}/run.mjs)\n`);
    put(workflows[workflow].asset, '// executable test example\n');
  }
  put('skills/rom/workflows.json', JSON.stringify({ profile_version: 1, platform: 'linux', workflows }));
  return { scratch, root, bundle, put, done: () => rmSync(scratch, { recursive: true, force: true }) };
}

test('portable bundle has deterministic content and verifies outside its source directory', () => {
  const f = fixture();
  try {
    assemble(f.root, f.bundle);
    const other = join(f.scratch, 'second-bundle');
    assemble(f.root, other);
    assert.equal(readFileSync(join(f.bundle, 'bundle.json'), 'utf8'), readFileSync(join(other, 'bundle.json'), 'utf8'));
    assert.equal(verify(f.bundle, f.root, 'resource').workflow, 'resource');
    assert.throws(() => assemble(f.root, f.bundle), /output exists/);
  } finally { f.done(); }
});

test('admission rejects missing changed and unsafe assets plus incompatible profiles or features', () => {
  for (const mutation of ['missing', 'changed', 'obsolete', 'feature', 'escape', 'link']) {
    const f = fixture();
    try {
      if (mutation === 'link') f.put('skills/rom/resource/SKILL.md', '---\nname: resource\ndescription: Use when a test is requested.\n---\n[Missing](absent.md)\n');
      if (mutation === 'link') {
        assert.throws(() => assemble(f.root, f.bundle), /broken local link/);
        continue;
      }
      assemble(f.root, f.bundle);
      const manifestPath = join(f.bundle, 'bundle.json');
      const manifest = JSON.parse(readFileSync(manifestPath));
      const asset = join(f.bundle, 'skills/rom/assets/resource/run.mjs');
      if (mutation === 'missing') rmSync(asset);
      if (mutation === 'changed') writeFileSync(asset, '// modified\n');
      if (mutation === 'obsolete') manifest.profile_version = 0;
      if (mutation === 'feature') manifest.workflows.resource.features.push('unsupported');
      if (mutation === 'escape') manifest.assets[0].path = '../escape';
      writeFileSync(manifestPath, JSON.stringify(manifest));
      assert.throws(() => verify(f.bundle, f.root, 'resource'), /asset|profile|feature|path/);
    } finally { f.done(); }
  }
});

test('admission rejects changed source identity and unknown workflows before execution', () => {
  const f = fixture();
  try {
    assemble(f.root, f.bundle);
    assert.throws(() => verify(f.bundle, f.root, 'unknown'), /workflow/);
    f.put('crates/rom/src/lib.rs', 'pub const TEST: u32 = 2;\n');
    assert.throws(() => verify(f.bundle, f.root, 'native'), /source/);
  } finally { f.done(); }
});

test('command rejects incompatible profile feature and missing asset before executing any example', () => {
  for (const mutation of ['profile', 'feature', 'removed-feature', 'substituted-asset', 'asset']) {
    const f = fixture();
    try {
      for (const module of ['verify', 'admission', 'assets', 'execution', 'files', 'rust-project', 'process']) {
        f.put(`scripts/skills/${module}.mjs`, readFileSync(new URL(`./${module}.mjs`, import.meta.url), 'utf8'));
      }
      f.put('skills/rom/assets/resource/run.mjs', "import { writeFileSync } from 'node:fs'; export async function run(context) {writeFileSync(context.root + '/executed', 'unexpected');}\n");
      assemble(f.root, f.bundle);
      const path = join(f.bundle, 'bundle.json');
      const manifest = JSON.parse(readFileSync(path));
      if (mutation === 'profile') manifest.profile_version = 0;
      if (mutation === 'feature') manifest.workflows.resource.features.push('unsupported');
      if (mutation === 'removed-feature') manifest.workflows.resource.features = [];
      if (mutation === 'substituted-asset') manifest.workflows.resource.asset = manifest.workflows.native.asset;
      if (mutation === 'asset') rmSync(join(f.bundle, 'skills/rom/assets/resource/run.mjs'));
      writeFileSync(path, JSON.stringify(manifest));
      const result = spawnSync(process.execPath, [join(f.bundle, 'tools/verify.mjs'), f.bundle, f.root, 'resource', '--run'], { encoding: 'utf8', timeout: 5000 });
      assert.equal(result.status, 1);
      assert.match(result.stderr, /profile|feature|asset/);
      assert.throws(() => readFileSync(join(f.root, 'executed')), /ENOENT/);
    } finally { f.done(); }
  }
});

test('admission rejects omitted indirect asset hashes and false lock identity metadata', () => {
  for (const mutation of ['indirect', 'lock']) {
    const f = fixture();
    try {
      f.put('scripts/skills/indirect.mjs', '// indirect helper\n');
      assemble(f.root, f.bundle);
      const path = join(f.bundle, 'bundle.json');
      const manifest = JSON.parse(readFileSync(path));
      if (mutation === 'indirect') {
        manifest.assets = manifest.assets.filter(asset => asset.path !== 'tools/indirect.mjs');
        writeFileSync(join(f.bundle, 'tools/indirect.mjs'), '// unverified modification\n');
      } else manifest.source.lock_sha256 = 'false';
      writeFileSync(path, JSON.stringify(manifest));
      assert.throws(() => verify(f.bundle, f.root, 'resource'), /asset|source/);
    } finally { f.done(); }
  }
});
