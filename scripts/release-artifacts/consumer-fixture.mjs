// Simulated retained-package fixture; no compiler or browser is executed.
import { mkdtempSync, mkdirSync, writeFileSync, cpSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { hash } from '../skills/files.mjs';
import { execFileSync } from 'node:child_process';
export function archiveFixture(directory) {
  execFileSync('tar', ['-czf', join(directory, 'studio-source.tar.gz'), '-C', join(directory, 'source-stage'), 'rom-studio-source'], { timeout: 10000 });
}
export function packageFixture(kind = 'controls', location) {
  const parent = location?.parent ?? mkdtempSync(join(tmpdir(), 'rom-consumer-files-'));
  const root = location?.root ?? join(parent, 'source'), directory = location?.directory ?? join(parent, 'evidence');
  const studio = join(root, 'studio'), installed = join(directory, 'consumer/node_modules/rom-studio');
  const fixturePath = kind === 'controls' ? 'public-controls' : 'mutation-recovery';
  mkdirSync(join(studio, 'src'), { recursive: true });
  mkdirSync(join(studio, 'tests', fixturePath, 'consumer'), { recursive: true });
  mkdirSync(installed, { recursive: true });
  for (const [path, text] of [['package.json', '{"name":"rom-studio"}'], ['src/index.ts', 'export const fixture = true;']]) {
    writeFileSync(join(studio, path), text);
  }
  for (const path of ['LICENSE', 'THIRD_PARTY_NOTICES.md']) writeFileSync(join(root, path), path);
  writeFileSync(join(studio, 'package-lock.json'), 'source lock');
  writeFileSync(join(studio, 'tests', fixturePath, 'consumer/package-lock.json'), 'consumer lock');
  cpSync(join(studio, 'package.json'), join(installed, 'package.json'));
  cpSync(join(studio, 'src'), join(installed, 'src'), { recursive: true });
  for (const path of ['LICENSE', 'THIRD_PARTY_NOTICES.md']) cpSync(join(root, path), join(installed, path));
  cpSync(join(studio, 'tests', fixturePath, 'consumer/package-lock.json'), join(directory, 'consumer/package-lock.json'));
  const staged = join(directory, 'source-stage/rom-studio-source');
  cpSync(installed, staged, { recursive: true });
  cpSync(join(studio, 'package-lock.json'), join(staged, 'package-lock.json'));
  archiveFixture(directory);
  const files = Object.fromEntries(['package.json', 'src/index.ts', 'LICENSE', 'THIRD_PARTY_NOTICES.md'].map(path => [path, hash(join(installed, path))]));
  const record = { completed: true, source_mode: 'extracted_release_source', source_input: studio,
    source_files: files, installed_package: installed, installed_source_matches: true,
    archive_sha256: hash(join(directory, 'studio-source.tar.gz')),
    consumer_lock_sha256: hash(join(directory, 'consumer/package-lock.json')) };
  if (kind === 'controls') Object.assign(record, { source_lock_sha256: hash(join(studio, 'package-lock.json')), dependency_bootstrap: 'locked_npm_ci', browser_executed: true });
  else Object.assign(record, { producer_lock_sha256: hash(join(studio, 'package-lock.json')), adapter: kind.slice(9), head: null, admitted: true });
  let fences = 0;
  return { lease: { root, fence() { fences++; } }, directory, record, installed, kind, fences: () => fences };
}

// Bind representative fixture bytes for orchestration tests, without compiling them.
export function recoveryFixtureEvidence(root, directory) {
  const inputs = ['consumer/src/index.ts', 'consumer/tests/case.ts', 'consumer/package.json',
    'consumer/package-lock.json', 'consumer/tsconfig.json', 'consumer/vite.config.ts', 'consumer/index.html',
    'runtime/proxy.mjs', 'runtime/server.mjs'];
  const generated = ['consumer/dist/bundle.js', 'consumer/http-dist/http.js'];
  const native = ['tests/recovery-host/Cargo.toml', 'tests/recovery-host/src/main.rs'];
  const write = (base, path) => {
    mkdirSync(join(base, path, '..'), { recursive: true });
    if (!existsSync(join(base, path))) writeFileSync(join(base, path), path);
  };
  for (const path of inputs) {
    write(directory, path);
    const source = path.startsWith('runtime/') ? path.slice(8) : path;
    const destination = join(root, 'studio/tests/mutation-recovery', source);
    mkdirSync(join(destination, '..'), { recursive: true });
    cpSync(join(directory, path), destination);
  }
  for (const path of generated) write(directory, path);
  for (const path of native) write(root, path);
  const verifier = 'studio/tests/mutation-recovery/verify.mjs';
  write(root, verifier);
  return {
    fixture_source_sha256: Object.fromEntries(native.map(path => [path, hash(join(root, path))])),
    consumer_fixture_sha256: Object.fromEntries([...inputs, ...generated].map(path => [path, hash(join(directory, path))])),
    verifier_sha256: hash(join(root, verifier)),
  };
}
