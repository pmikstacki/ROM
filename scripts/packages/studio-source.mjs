// Exact copied frontend inputs, independent of checkout-only build output.
import { cpSync, existsSync, lstatSync, mkdirSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { applicationInputs } from './application.mjs';
import { digest, hash } from '../skills/files.mjs';
import { isPrivateSourcePath } from './source-policy.mjs';

const generated = new Set(['node_modules', 'dist', '.component-dist', '.demo-dist', '.demo-field-dist', 'test-results', 'playwright-report', '.git']);
export const isGeneratedStudioPath = path => generated.has(path.split('/')[0]);

function sourcePaths(directory) {
  if (!lstatSync(directory).isDirectory() || lstatSync(directory).isSymbolicLink()) throw Error('invalid Studio directory');
  for (const path of ['package.json', 'package-lock.json', 'index.html', 'vite.config.ts', 'svelte.config.js', 'tsconfig.json', 'src']) {
    const full = join(directory, path);
    if (!existsSync(full) || lstatSync(full).isSymbolicLink()) throw Error(`missing or symbolic Studio input: ${path}`);
    if (path === 'src' ? !lstatSync(full).isDirectory() : !lstatSync(full).isFile()) throw Error(`invalid Studio input: ${path}`);
  }
  return readdirSync(directory).filter(path => !isGeneratedStudioPath(path)).sort();
}

export function studioSource(root) {
  const directory = join(root, 'studio');
  const paths = sourcePaths(directory);
  const pkg = JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8'));
  const lock = JSON.parse(readFileSync(join(directory, 'package-lock.json'), 'utf8'));
  const locked = lock.packages?.[''];
  if (pkg.name !== 'rom-studio' || pkg.private !== true || typeof pkg.version !== 'string' ||
      lock.lockfileVersion !== 3 || lock.name !== pkg.name || lock.version !== pkg.version ||
      locked?.name !== pkg.name || locked?.version !== pkg.version ||
      ['dependencies', 'devDependencies', 'optionalDependencies'].some(name =>
        !isDeepStrictEqual(pkg[name] ?? {}, locked[name] ?? {}))) throw Error('incompatible Studio npm lock');
  const files = applicationInputs(directory, paths);
  if (Object.keys(files).some(isPrivateSourcePath)) throw Error('private Studio input');
  return { name: pkg.name, package_version: pkg.version, package_json_sha256: hash(join(directory, 'package.json')),
    package_lock_sha256: hash(join(directory, 'package-lock.json')), files, sha256: digest(files) };
}

export function copyStudio(root, destination) {
  const before = studioSource(root);
  mkdirSync(destination);
  const output = join(destination, 'studio');
  mkdirSync(output);
  for (const path of sourcePaths(join(root, 'studio'))) cpSync(join(root, 'studio', path), join(output, path), { recursive: true, dereference: false });
  if (!isDeepStrictEqual(studioSource(destination), before)) throw Error('Studio copy identity mismatch');
  return output;
}
