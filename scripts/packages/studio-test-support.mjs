// Synthetic frontend fixtures do not claim browser or provider acceptance.
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';

export function putStudio(root, name, text) {
  const path = join(root, 'studio', name);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text);
}
export function frontendFixture(root = mkdtempSync(join(tmpdir(), 'rom-studio-package-test-')), version = '0.0.2') {
  const pkg = { name: 'rom-studio', version, private: true, dependencies: { svelte: '5.57.1' }, devDependencies: { vite: '8.3.2' } };
  putStudio(root, 'package.json', JSON.stringify(pkg));
  putStudio(root, 'package-lock.json', JSON.stringify({ name: pkg.name, version: pkg.version, lockfileVersion: 3, packages: { '': pkg } }));
  for (const path of ['index.html', 'src/App.svelte', 'vite.config.ts', 'svelte.config.js', 'tsconfig.json', '.gitignore', 'tests/input.ts']) putStudio(root, path, 'source');
  return { root, put: (name, text) => putStudio(root, name, text) };
}
export function assetFixture(root) {
  mkdirSync(join(root, 'assets'), { recursive: true });
  writeFileSync(join(root, 'index.html'), '<html><script type="module" src="/rom-studio/assets/main.js"></script><link rel="stylesheet" href="/rom-studio/assets/main.css"></html>');
  writeFileSync(join(root, 'assets/main.js'), 'console.log("fixture");');
  writeFileSync(join(root, 'assets/main.css'), 'body{}');
  return root;
}
export async function fixtureRunner(program, args, options) {
  if (program === 'npm' && args[0] === 'run' && args[1] === 'build') assetFixture(join(options.cwd, 'dist'));
  return { code: 0, stdout: 'finite synthetic fixture passed\n', stderr: '', timedOut: false };
}
