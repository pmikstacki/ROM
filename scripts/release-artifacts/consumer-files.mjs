// Compare retained package bytes without executing package or consumer programs.
import { lstatSync, readdirSync, realpathSync, existsSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { isAbsolute, resolve, join } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { hash, safePath } from '../skills/files.mjs';
import { requireConsumerPackage } from './consumer-package.mjs';
import { extractStudioArchive } from './studio-source-archive.mjs';

import { consumerOrigins } from './consumer-origins.mjs';
const kinds = ['controls', 'recovery-sqlite', 'recovery-redb'];
function canonicalDirectory(path) {
  if (typeof path !== 'string' || !isAbsolute(path) || resolve(path) !== path ||
      realpathSync(path) !== path || !lstatSync(path).isDirectory()) throw Error('invalid evidence directory');
}
function fileHash(root, path) {
  const full = safePath(root, path), stat = lstatSync(full);
  if (!stat.isFile() || stat.nlink !== 1 || stat.size > 64 * 1024 * 1024) throw Error('invalid evidence file');
  return hash(full);
}
function packageFiles(root, fallback) {
  const result = {};
  let count = 0, bytes = 0;
  function visit(path, depth = 0) {
    if (depth > 32 || ++count > 100_000) throw Error('package inventory exceeds bound');
    const full = safePath(root, path), stat = lstatSync(full);
    if (stat.isDirectory()) {
      for (const entry of readdirSync(full).sort()) visit(`${path}/${entry}`, depth + 1);
    } else {
      bytes += stat.size;
      if (bytes > 256 * 1024 * 1024) throw Error('package bytes exceed bound');
      result[path] = fileHash(root, path);
    }
  }
  visit('package.json'); visit('src');
  for (const path of ['LICENSE', 'THIRD_PARTY_NOTICES.md']) {
    if (existsSync(join(root, path))) visit(path);
    else if (fallback) result[path] = fileHash(fallback, path);
    else throw Error('missing package notice');
  }
  return result;
}

/** Internal seam: a verified extraction must establish lease.root before this comparison. */
export function requireConsumerFiles(lease, directory, kind, record, context) {
  try {
    if (!lease || typeof lease.fence !== 'function' || !kinds.includes(kind)) throw Error('invalid source lease');
    canonicalDirectory(lease.root); canonicalDirectory(directory);
    lease.fence();
    const source = safePath(lease.root, 'studio'); canonicalDirectory(source);
    const installed = safePath(directory, 'consumer/node_modules/rom-studio'); canonicalDirectory(installed);
    const expected = packageFiles(source, lease.root), actual = packageFiles(installed);
    if (!isDeepStrictEqual(expected, actual)) throw Error('installed package bytes differ');
    const fixture = kind === 'controls' ? 'public-controls' : 'mutation-recovery';
    const consumerLock = fileHash(lease.root, `studio/tests/${fixture}/consumer/package-lock.json`);
    if (fileHash(directory, 'consumer/package-lock.json') !== consumerLock) throw Error('consumer lock bytes differ');
    const archive = safePath(directory, 'studio-source.tar.gz'), archiveDigest = fileHash(directory, 'studio-source.tar.gz');
    const sourceLock = fileHash(lease.root, 'studio/package-lock.json');
    const scratch = mkdtempSync(join(tmpdir(), 'rom-consumer-archive-check-'));
    try {
      const archived = extractStudioArchive(archive, scratch);
      if (!isDeepStrictEqual(packageFiles(archived), expected) || fileHash(archived, 'package-lock.json') !== sourceLock)
        throw Error('archived package bytes differ');
    } finally { rmSync(scratch, { recursive: true, force: true }); }
    if (fileHash(directory, 'studio-source.tar.gz') !== archiveDigest) throw Error('archive changed during validation');
    requireConsumerPackage(record, { kind, ...consumerOrigins(lease,directory,kind,context), source_files: expected,
      source_lock_sha256: sourceLock, consumer_lock_sha256: consumerLock,
      archive_sha256: archiveDigest });
    lease.fence();
    return record;
  } catch (error) {
    throw Error('invalid consumer file evidence', { cause: error });
  }
}
