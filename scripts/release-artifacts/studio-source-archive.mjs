// Bound and validate candidate source members before using the shared release extractor.
import { lstatSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname } from 'node:path';
import { extract } from './archives.mjs';

export const sourcePrefix = 'rom-studio-source';
const limits = { maxEntries: 20000, maxBytes: 64 * 1024 * 1024 };
const maxCompressedBytes = 32 * 1024 * 1024;
export function assertStudioArchiveFile(path) {
  const info = lstatSync(path);
  if (!info.isFile() || info.isSymbolicLink() || info.size > maxCompressedBytes) throw Error('invalid source archive file or compressed byte bound');
}
export function validateStudioArchiveListing(paths, details, bounds = limits) {
  if (!paths.length || paths.length > bounds.maxEntries || details.length !== paths.length) throw Error('source archive member bound');
  const members = new Set();
  let bytes = 0;
  for (const [i, path] of paths.entries()) {
    if (members.has(path)) throw Error('duplicate archive member');
    members.add(path);
    const parts = path.split('/');
    if (parts.at(-1) === '') parts.pop();
    if (/[\\\x00-\x1f\x7f]/.test(path) || !path.startsWith(`${sourcePrefix}/`) || parts.some(part => !part || part === '..' || part === '.')) throw Error('invalid source archive path');
    const suffix = path.slice(sourcePrefix.length + 1);
    if (suffix && !['package.json', 'package-lock.json', 'LICENSE', 'THIRD_PARTY_NOTICES.md'].includes(suffix) && !suffix.startsWith('src/')) throw Error('unadmitted source archive member');
    if (!['-', 'd'].includes(details[i][0])) throw Error('unsupported source archive entry');
    const size = /^\S+\s+\S+\s+(\d+)\s/.exec(details[i]);
    if (!size || !Number.isSafeInteger(Number(size[1]))) throw Error('invalid source archive byte count');
    bytes += Number(size[1]);
    if (!Number.isSafeInteger(bytes) || bytes > bounds.maxBytes) throw Error('source archive byte bound');
  }
  for (const required of ['', 'package.json', 'package-lock.json', 'LICENSE', 'THIRD_PARTY_NOTICES.md', 'src/']) {
    if (!members.has(`${sourcePrefix}/${required}`)) throw Error('missing required source archive member');
  }
}
export function extractStudioArchive(archive, parent) {
  assertStudioArchiveFile(archive);
  // The expanded tar budget also rejects compressed amplification before any file write.
  execFileSync('gzip', ['-dc', archive], { cwd: dirname(archive), timeout: 10000, maxBuffer: limits.maxBytes });
  const capture = args => execFileSync('tar', args, { cwd: parent, encoding: 'utf8', timeout: 10000, maxBuffer: 8 * 1024 * 1024 }).trim().split('\n');
  const paths = capture(['-tzf', archive]);
  const details = capture(['-tvzf', archive, '--numeric-owner']);
  validateStudioArchiveListing(paths, details);
  return extract(archive, parent, sourcePrefix);
}
