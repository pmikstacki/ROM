import assert from 'node:assert/strict';
import { test } from 'node:test';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, symlinkSync, linkSync, existsSync, truncateSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { extractCrate, validateCrateSizes } from './crate-extraction.mjs';
const run = (args, cwd) => execFileSync('tar', args, { cwd, timeout: 10000, maxBuffer: 8 * 1024 * 1024, encoding: 'utf8' });
function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'rom-cargo-crate-extract-')), prefix = 'rom-ai-0.0.3';
  mkdirSync(join(root, prefix, 'src'), { recursive: true }); mkdirSync(join(root, 'output'));
  writeFileSync(join(root, prefix, 'Cargo.toml'), '[package]\nname = "rom-ai"\nversion = "0.0.3"\n');
  writeFileSync(join(root, prefix, 'src/lib.rs'), 'pub struct Public;\n');
  return { root, prefix, output: join(root, 'output'), archive: join(root, 'fixture.crate') };
}
test('genuine Cargo layout extracts file entries without an explicit root-directory record', () => {
  const f = fixture();
  run(['-czf', f.archive, `${f.prefix}/Cargo.toml`, `${f.prefix}/src/lib.rs`], f.root);
  const listing = run(['-tzf', f.archive], f.root).trim().split('\n');
  assert.deepEqual(listing, ['rom-ai-0.0.3/Cargo.toml', 'rom-ai-0.0.3/src/lib.rs']);
  assert.equal(listing.includes(f.prefix + '/'), false);
  const result = extractCrate(f.archive, f.output, f.prefix);
  assert.equal(readFileSync(join(result, 'src/lib.rs'), 'utf8'), 'pub struct Public;\n');
  assert.throws(() => extractCrate(f.archive, f.output, f.prefix), /exist|exclusive|occupied/);
});
test('explicit directory record remains supported', () => {
  const f = fixture(); run(['-czf', f.archive, f.prefix], f.root);
  assert.equal(readFileSync(join(extractCrate(f.archive, f.output, f.prefix), 'Cargo.toml'), 'utf8'), '[package]\nname = "rom-ai"\nversion = "0.0.3"\n');
});
test('duplicate files fail before creating the package output', () => {
  const f = fixture(); run(['-czf', f.archive, `${f.prefix}/Cargo.toml`, `${f.prefix}/Cargo.toml`], f.root);
  assert.throws(() => extractCrate(f.archive, f.output, f.prefix), /duplicate/);
  assert.equal(existsSync(join(f.output, f.prefix)), false);
});
test('symbolic and hard links fail before extraction', () => {
  for (const kind of ['symlink', 'hardlink']) {
    const f = fixture(), original = join(f.root, f.prefix, 'src/lib.rs'), extra = join(f.root, f.prefix, 'src/link.rs');
    if (kind === 'symlink') symlinkSync('/outside/private', extra); else linkSync(original, extra);
    run(['-czf', f.archive, `${f.prefix}/src/lib.rs`, `${f.prefix}/src/link.rs`], f.root);
    assert.throws(() => extractCrate(f.archive, f.output, f.prefix), /unsupported/);
    assert.equal(existsSync(join(f.output, f.prefix)), false);
  }
});
test('outside-prefix and traversal names fail before extraction', () => {
  for (const replacement of ['outside/Cargo.toml', 'rom-ai-0.0.3/../outside']) {
    const f = fixture();
    run(['-czf', f.archive, '--transform', `s|${f.prefix}/Cargo.toml|${replacement}|`, `${f.prefix}/Cargo.toml`], f.root);
    assert.throws(() => extractCrate(f.archive, f.output, f.prefix), /path/);
    assert.equal(existsSync(join(f.output, f.prefix)), false);
  }
});

test('tiny GNU sparse archive with oversized logical payload is rejected without expanding its file', () => {
  const f = fixture(), sparse = join(f.root, f.prefix, 'src/lib.rs');
  truncateSync(sparse, 129 * 1024 * 1024);
  run(['--sparse', '-czf', f.archive, `${f.prefix}/src/lib.rs`], f.root);
  assert.ok(statSync(f.archive).size < 1024 * 1024);
  const detailed = run(['-tvzf', f.archive, '--numeric-owner'], f.root).trimEnd().split('\n');
  assert.throws(() => validateCrateSizes(detailed), /size budget/);
  assert.throws(() => extractCrate(f.archive, f.output, f.prefix), /size budget/);
  assert.equal(existsSync(join(f.output, f.prefix)), false);
});

test('aggregate logical file sizes fail before extraction even when each file fits', () => {
  assert.throws(() => validateCrateSizes([
    '-rw-r--r-- 0/0 67108865 2026-10-08 12:00 rom-ai-0.0.3/a',
    '-rw-r--r-- 0/0 67108865 2026-10-08 12:00 rom-ai-0.0.3/b',
  ]), /size budget/);
});

test('inherited TAR_OPTIONS cannot change the admitted package extraction', () => {
  const f = fixture(); run(['-czf', f.archive, `${f.prefix}/Cargo.toml`, `${f.prefix}/src/lib.rs`], f.root);
  const prior = process.env.TAR_OPTIONS; process.env.TAR_OPTIONS = '--exclude=*/src/lib.rs';
  try {
    const extracted = extractCrate(f.archive, f.output, f.prefix);
    assert.equal(readFileSync(join(extracted, 'src/lib.rs'), 'utf8'), 'pub struct Public;\n');
  } finally { if (prior === undefined) delete process.env.TAR_OPTIONS; else process.env.TAR_OPTIONS = prior; }
});

test('special FIFO payload is rejected before extraction', () => {
  const f = fixture(), fifo = join(f.root, f.prefix, 'fifo');
  execFileSync('mkfifo', [fifo], { timeout: 10000 });
  run(['-czf', f.archive, `${f.prefix}/fifo`], f.root);
  assert.throws(() => extractCrate(f.archive, f.output, f.prefix), /unsupported/);
  assert.equal(existsSync(join(f.output, f.prefix)), false);
});
