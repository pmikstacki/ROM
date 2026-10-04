// Committed-source admission and fencing, independent of selected skill hashes.
import { execFileSync } from 'node:child_process';
import { lstatSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { hash, identity, relativePath } from '../skills/files.mjs';
import { isGeneratedStudioPath } from '../packages/studio-source.mjs';
import { isPrivateSourcePath } from '../packages/source-policy.mjs';

export const capture = (program, args, cwd) => execFileSync(program, args, {
  cwd, encoding: 'utf8', timeout: 10000, maxBuffer: 32 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'],
});
function excluded(path) {
  const parts = path.split('/');
  return /[\x00-\x1f\x7f]/.test(path)
    || parts.some(part => ['target', 'node_modules', '.git', '.superpowers'].includes(part))
    || (parts[0] === 'studio' && isGeneratedStudioPath(parts.slice(1).join('/')))
    || isPrivateSourcePath(path);
}
export function snapshot(root) {
  root = resolve(root);
  let revision, tree, status, entries;
  try {
    if (capture('git', ['rev-parse', '--show-toplevel'], root).trim() !== root) throw Error('root mismatch');
    revision = capture('git', ['rev-parse', 'HEAD'], root).trim();
    tree = capture('git', ['rev-parse', 'HEAD^{tree}'], root).trim();
    status = capture('git', ['status', '--porcelain', '--untracked-files=normal'], root);
    entries = capture('git', ['ls-tree', '-rz', '--full-tree', 'HEAD'], root).split('\0').filter(Boolean);
  } catch { throw Error('release requires a usable Git checkout'); }
  if (status) throw Error('release requires clean source');
  const files = entries.map(entry => {
    const match = /^(100644|100755) blob ([0-9a-f]+)\t(.+)$/s.exec(entry);
    if (!match) throw Error('unsupported source entry');
    const path = relativePath(match[3]);
    if (excluded(path)) throw Error('excluded source path');
    if (!lstatSync(join(root, path)).isFile()) throw Error('unsupported source entry');
    return { path, mode: match[1], sha256: hash(join(root, path)) };
  });
  const profile = JSON.parse(readFileSync(join(root, 'extensions/native-alpha-v1.json'), 'utf8'));
  if (profile.profile_version !== 1 || profile.publication_enabled !== false || !/^\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?$/.test(profile.package_version)) throw Error('unsupported release profile');
  const objectFormat = capture('git', ['rev-parse', '--show-object-format'], root).trim();
  return { revision, tree, object_format: objectFormat, lock_sha256: hash(join(root, 'Cargo.lock')), package_version: profile.package_version, profile, selected: identity(root), files };
}
export function fence(root, expected) {
  let current;
  try { current = snapshot(root); }
  catch (error) { if (error.message.includes('clean source')) throw Error('source must remain clean'); throw error; }
  if (JSON.stringify(current) !== JSON.stringify(expected)) throw Error('source changed during release');
}
export function toolchain(root) {
  return Object.fromEntries([
    ['rustc', ['--version']], ['cargo', ['--version']], ['node', ['--version']], ['npm', ['--version']],
    ['git', ['--version']], ['tar', ['--version']], ['gzip', ['--version']], ['mv', ['--version']], ['findmnt', ['--version']],
  ].map(([program, args]) => [program, capture(program, args, root).split('\n')[0]]));
}
