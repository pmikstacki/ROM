//! Controlled bundle paths, source identities and local Markdown references.
import { createHash } from 'node:crypto';
import { lstatSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, resolve, relative, sep } from 'node:path';

export function relativePath(path) {
  if (typeof path !== 'string' || !path || path.includes('\\') || path.startsWith('/') || path.split('/').some(part => !part || part === '..' || part === '.')) throw Error('invalid asset path');
  return path;
}
export function safePath(root, path) {
  relativePath(path);
  const full = resolve(root, path);
  if (!full.startsWith(resolve(root) + sep)) throw Error('invalid asset path');
  let current = resolve(root);
  for (const part of path.split('/')) {
    current = join(current, part);
    if (lstatSync(current).isSymbolicLink()) throw Error('symbolic asset path');
  }
  return full;
}
export const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
export const digest = value => createHash('sha256').update(JSON.stringify(value)).digest('hex');
export function files(root, dir) {
  const out = [];
  function walk(path) {
    const full = join(root, path);
    for (const entry of readdirSync(full, { withFileTypes: true }).sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0)) {
      if (['target', 'node_modules', '.git', '.superpowers'].includes(entry.name)) continue;
      const next = path ? `${path}/${entry.name}` : entry.name;
      if (entry.isSymbolicLink()) throw Error('symbolic asset path');
      if (entry.isDirectory()) walk(next);
      else if (entry.isFile()) out.push(next);
    }
  }
  walk(dir);
  return out.sort();
}
export function identity(root) {
  const paths = ['Cargo.toml', 'Cargo.lock'];
  for (const dir of ['crates', 'demo', 'examples']) {
    try { paths.push(...files(root, dir).filter(path => path.endsWith('.rs') || path.endsWith('/Cargo.toml'))); }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
  }
  const entries = paths.sort().map(path => ({ path, sha256: hash(safePath(root, path)) }));
  return { files: entries, sha256: digest(entries), lock_sha256: hash(safePath(root, 'Cargo.lock')) };
}
export function localLinks(root, path) {
  const text = readFileSync(safePath(root, path), 'utf8');
  for (const match of text.matchAll(/!?\[[^\]]*\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g)) {
    const link = match[1];
    if (/^[a-z][a-z0-9+.-]*:/i.test(link) || link.startsWith('#')) continue;
    const destination = link.split('#')[0];
    if (!destination || destination.startsWith('/')) throw Error('broken local link');
    const normalized = relative(resolve(root), resolve(root, dirname(path), destination)).split(sep).join('/');
    try { safePath(root, normalized); } catch { throw Error('broken local link'); }
  }
}
