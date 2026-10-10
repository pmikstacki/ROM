// Pure admission for a supplied source and frozen fixture graph. No acquisition.
import { mkdirSync, lstatSync, realpathSync, readdirSync, openSync, closeSync, fstatSync, readSync, constants } from 'node:fs';
import { resolve, join } from 'node:path';
import { createHash } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';

export const SHA256 = /^[a-f0-9]{64}$/;
export const plain = value => value !== null && typeof value === 'object' && !Array.isArray(value) && Object.getPrototypeOf(value) === Object.prototype;
export const exactKeys = (value, names) => plain(value) && isDeepStrictEqual(Object.keys(value).sort(), [...names].sort());

export function reserveEvidence(path) {
  if (typeof path !== 'string' || !path || resolve(path) !== path || path === '/') throw Error('invalid evidence path');
  try { mkdirSync(path, { mode: 0o700 }); }
  catch (error) { if (error.code === 'EEXIST') throw Error('evidence output already exists'); throw error; }
  return path;
}

export function requireFrozenLock(expected, actual, mode) {
  if (mode !== 'locked_npm_ci') throw Error('admission requires locked npm ci');
  if (!plain(expected) || !plain(actual) || !isDeepStrictEqual(expected, actual)) throw Error('dependency graph drift');
}

export function verifySource(root, manifest) {
  if (typeof root !== 'string' || resolve(root) !== root || realpathSync(root) !== root || !lstatSync(root).isDirectory()) throw Error('invalid source path');
  if (!plain(manifest) || !Object.keys(manifest).length || Object.keys(manifest).length > 20_000) throw Error('invalid source manifest');
  const inventory = []; let nodes = 0;
  function visit(path) {
    if (++nodes > 40_000) throw Error('source inventory limit');
    const info = lstatSync(join(root, path));
    if (info.isSymbolicLink()) throw Error('source symbolic link rejected');
    if (info.isDirectory()) {
      for (const entry of readdirSync(join(root, path)).sort()) visit(path ? `${path}/${entry}` : entry);
    } else if (info.isFile()) inventory.push(path);
    else throw Error('source inventory non-regular file');
  }
  // Validate paths before inspecting the actual inventory, to preserve useful failure categories.
  for (const path of Object.keys(manifest)) {
    if (!/^[A-Za-z0-9_.\/-]+$/.test(path) || path.split('/').some(p => !p || p === '.' || p === '..')) throw Error('invalid source path');
  }
  visit('');
  if (!isDeepStrictEqual(inventory.sort(), Object.keys(manifest).sort())) throw Error('source inventory mismatch');
  const actual = []; let total = 0;
  for (const [path, expected] of Object.entries(manifest).sort()) {
    const parts = path.split('/');
    if (!/^[A-Za-z0-9_.\/-]+$/.test(path) || parts.some(p => !p || p === '.' || p === '..') || !SHA256.test(expected)) throw Error('invalid source path or digest');
    let current = root;
    for (const part of parts) {
      current = join(current, part);
      if (lstatSync(current).isSymbolicLink()) throw Error('source symbolic link rejected');
    }
    const fd = openSync(current, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    try {
      const before = fstatSync(fd);
      if (!before.isFile() || before.size > 8 * 1024 * 1024 || total + before.size > 128 * 1024 * 1024) throw Error('source file limit');
      const bytes = Buffer.alloc(before.size + 1); let count = 0;
      while (count < bytes.length) { const read = readSync(fd, bytes, count, bytes.length - count, null); if (!read) break; count += read; }
      const after = fstatSync(fd);
      if (count !== before.size || after.size !== before.size || after.mtimeMs !== before.mtimeMs || after.ctimeMs !== before.ctimeMs) throw Error('source mismatch during read');
      total += count;
      const digest = createHash('sha256').update(bytes.subarray(0, count)).digest('hex');
      if (digest !== expected) throw Error(`source mismatch: ${path}`);
      actual.push([path, digest]);
    } finally { closeSync(fd); }
  }
  return Object.fromEntries(actual);
}

export function requireProviderIdentity(expected, actual) {
  const names = ['provider', 'version', 'image', 'platform', 'source_revision'];
  if (!exactKeys(expected, names) || !exactKeys(actual, names) || !isDeepStrictEqual(expected, actual) ||
      expected.provider !== 'authentik' || !/^\d{4}\.\d+\.\d+$/.test(expected.version) ||
      !/^ghcr\.io\/goauthentik\/server@sha256:[a-f0-9]{64}$/.test(expected.image) ||
      !/^linux\/(?:amd64|arm64)$/.test(expected.platform) || !/^[a-f0-9]{40}$/.test(expected.source_revision)) throw Error('invalid provider identity');
  return actual;
}
