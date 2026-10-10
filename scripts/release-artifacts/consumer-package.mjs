// Static identity binding does not establish source trust or prove browser execution.
import { isDeepStrictEqual } from 'node:util';
import { isAbsolute, resolve, posix } from 'node:path';

const keys = ['kind', 'source_input', 'source_files', 'source_lock_sha256', 'consumer_lock_sha256', 'installed_package', 'archive_sha256'];
const kinds = ['controls', 'recovery-sqlite', 'recovery-redb'];
const digest = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const canonical = value => typeof value === 'string' && isAbsolute(value) && resolve(value) === value && value !== '/';
function inventory(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const entries = Object.entries(value);
  return entries.length <= 100_000 &&
    ['package.json', 'LICENSE', 'THIRD_PARTY_NOTICES.md'].every(path => digest(value[path])) &&
    entries.some(([path]) => path.startsWith('src/')) && entries.every(([path, hash]) =>
      path.length <= 4096 && !path.includes('\\') && !path.includes('\0') &&
      !posix.isAbsolute(path) && posix.normalize(path) === path &&
      path.split('/').every(part => part && part !== '.' && part !== '..') && digest(hash));
}

/** expected must come from admitted source and retained bytes, not the report being checked. */
export function requireConsumerPackage(record, expected) {
  const fail = () => { throw Error('invalid consumer package evidence'); };
  if (!expected || typeof expected !== 'object' || Array.isArray(expected) ||
      !isDeepStrictEqual(Object.keys(expected).sort(), [...keys].sort()) ||
      !kinds.includes(expected.kind) || !canonical(expected.source_input) ||
      !canonical(expected.installed_package) || !inventory(expected.source_files) ||
      !['source_lock_sha256', 'consumer_lock_sha256', 'archive_sha256'].every(key => digest(expected[key]))) fail();
  if (!record || record.completed !== true || record.source_mode !== 'extracted_release_source' ||
      record.source_input !== expected.source_input || record.installed_package !== expected.installed_package ||
      record.installed_source_matches !== true || record.archive_sha256 !== expected.archive_sha256 ||
      record.consumer_lock_sha256 !== expected.consumer_lock_sha256 ||
      !isDeepStrictEqual(record.source_files, expected.source_files)) fail();
  if (expected.kind === 'controls') {
    if (record.source_lock_sha256 !== expected.source_lock_sha256 ||
        record.dependency_bootstrap !== 'locked_npm_ci' || record.browser_executed !== true) fail();
  } else if (record.producer_lock_sha256 !== expected.source_lock_sha256 || record.admitted !== true ||
      record.adapter !== expected.kind.slice('recovery-'.length) || record.head !== null) fail();
  return record;
}
