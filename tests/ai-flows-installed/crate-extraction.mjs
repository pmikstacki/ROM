// Cargo crate archives need not contain an explicit root-directory record.
import { execFileSync } from 'node:child_process';
import { lstatSync, mkdirSync, readFileSync, realpathSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { gunzipSync } from 'node:zlib';
import { relativePath } from '../../scripts/skills/files.mjs';

export function validateCrateSizes(detailed) {
  return detailed.reduce((total, line) => {
    const size = /^[d-]\S+\s+\d+\/\d+\s+(\d+)\s/.exec(line)?.[1];
    if (size === undefined) throw Error('invalid crate archive size metadata');
    const logical = BigInt(size), next = total + logical;
    if (logical > 128n * 1024n * 1024n || next > 128n * 1024n * 1024n) throw Error('crate archive logical size budget exceeded');
    return next;
  }, 0n);
}

export function extractCrate(archive, parent, prefix) {
  if (!/^[A-Za-z0-9][A-Za-z0-9._+-]*$/.test(prefix)) throw Error('invalid archive prefix');
  if (realpathSync(parent) !== resolve(parent) || !lstatSync(parent).isDirectory()) throw Error('invalid extraction parent');
  const stat = lstatSync(archive);
  if (!stat.isFile() || stat.size > 128 * 1024 * 1024) throw Error('invalid or oversized crate archive');
  // All commands consume one immutable bounded snapshot, not a replaceable archive pathname.
  const bytes = gunzipSync(readFileSync(archive), { maxOutputLength: 128 * 1024 * 1024 });
  const tar = args => execFileSync('tar', args, {
    cwd: parent, input: bytes, encoding: 'utf8', timeout: 10000,
    env: { PATH: process.env.PATH, LC_ALL: 'C', LANG: 'C' },
    maxBuffer: 8 * 1024 * 1024, stdio: ['pipe', 'pipe', 'pipe'],
  });
  const listing = tar(['-tf', '-', '--quoting-style=escape']).trimEnd().split('\n');
  if (!listing[0]) throw Error('empty crate archive');
  const seen = new Set();
  for (const path of listing) {
    const canonical = path.endsWith('/') ? path.slice(0, -1) : path;
    if (/[\x00-\x1f\x7f\\]/.test(path) || (canonical !== prefix && !canonical.startsWith(prefix + '/'))) throw Error('invalid crate archive path');
    relativePath(canonical);
    if (seen.has(canonical)) throw Error('duplicate crate archive path');
    seen.add(canonical);
  }
  const detailed = tar(['-tvf', '-', '--quoting-style=escape', '--numeric-owner']).trimEnd().split('\n');
  if (detailed.length !== listing.length || detailed.some(line => !['-', 'd'].includes(line[0]))) throw Error('unsupported crate archive entry');
  validateCrateSizes(detailed);
  const files = new Set();
  for (let index = 0; index < listing.length; index++) {
    const path = listing[index].replace(/\/$/, '');
    if (path === prefix && detailed[index][0] !== 'd') throw Error('invalid crate archive root path');
    if (detailed[index][0] === '-') files.add(path);
  }
  if (!files.size) throw Error('empty crate archive payload');
  for (const path of seen) {
    const parts = path.split('/');
    while (parts.pop(), parts.length) if (files.has(parts.join('/'))) throw Error('file conflicts with crate directory path');
  }
  // mkdir is exclusive. Accepted paths are stripped into this new directory only.
  const output = join(parent, prefix); mkdirSync(output, { mode: 0o700 });
  tar(['-xf', '-', '-C', output, '--strip-components=1', '--no-same-owner', '--no-same-permissions', '--keep-old-files']);
  return output;
}
