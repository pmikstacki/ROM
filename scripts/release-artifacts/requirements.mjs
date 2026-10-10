// Static metadata and retained-byte checks; actual requirement coverage needs independent review.
import { isDeepStrictEqual } from 'node:util';
import { lstatSync } from 'node:fs';
import { safePath, hash } from '../skills/files.mjs';
import { COMMAND_LOG_BYTES } from './command-limits.mjs';

const ids = Object.freeze(Array.from({ length: 14 }, (_, i) => `R${i + 1}`));
const exact = (value, keys) => value && typeof value === 'object' && !Array.isArray(value) && isDeepStrictEqual(Object.keys(value).sort(), [...keys].sort());
const sha = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const text = value => typeof value === 'string' && Buffer.byteLength(value) <= 16 * 1024;
const fail = () => { throw Error('invalid release requirement evidence'); };

function requireReference(ref, directory, kind) {
  if (!exact(ref, ['path', 'sha256', 'bytes', 'kind']) || !sha(ref.sha256) ||
      !Number.isSafeInteger(ref.bytes) || ref.bytes <= 0 || ref.bytes > COMMAND_LOG_BYTES ||
      !['executed', 'inspected', 'candidate', 'review'].includes(ref.kind) ||
      (kind && ref.kind !== kind)) fail();
  const path = safePath(directory, ref.path), stat = lstatSync(path);
  if (!stat.isFile() || stat.nlink !== 1 || stat.size !== ref.bytes || hash(path) !== ref.sha256) fail();
}

/** No npm, host, compiler, browser, or artifact program is executed by this function. */
export function requireReleaseRequirements(record, sourceIdentity, directory, options = {}) {
  let result;
  try {
    if (!exact(options, []) && !exact(options, ['requireComplete'])) fail();
    if (options.requireComplete !== undefined && typeof options.requireComplete !== 'boolean') fail();
    if (!sha(sourceIdentity) || !exact(record, ['schema_version', 'source_identity', 'requirements']) ||
        record.schema_version !== 1 || record.source_identity !== sourceIdentity ||
        !Array.isArray(record.requirements) || record.requirements.length !== ids.length) fail();
    const seen = new Set(), pending = [], failed = [];
    let accepted = 0;
    for (const entry of record.requirements) {
      if (!exact(entry, ['id', 'state', 'source_identity', 'evidence', 'measured_scope', 'limitations', 'review']) ||
          !ids.includes(entry.id) || seen.has(entry.id) || entry.source_identity !== sourceIdentity ||
          !['pending', 'failed', 'accepted'].includes(entry.state) ||
          !Array.isArray(entry.evidence) || entry.evidence.length > 32 ||
          !text(entry.measured_scope) || !text(entry.limitations)) fail();
      seen.add(entry.id);
      const paths = new Set();
      for (const ref of entry.evidence) {
        requireReference(ref, directory);
        if (ref.kind === 'review' || paths.has(ref.path)) fail();
        paths.add(ref.path);
      }
      if (entry.review !== null) {
        requireReference(entry.review, directory, 'review');
        if (paths.has(entry.review.path)) fail();
      }
      if (entry.state === 'accepted') {
        if (!entry.evidence.length || entry.evidence.some(ref => ref.kind !== 'executed') ||
            !entry.review || !entry.measured_scope.trim()) fail();
        accepted++;
      } else (entry.state === 'pending' ? pending : failed).push(entry.id);
    }
    const order = list => list.sort((a, b) => ids.indexOf(a) - ids.indexOf(b));
    result = { complete: accepted === ids.length, accepted, pending: order(pending), failed: order(failed) };
  } catch (error) {
    if (error.message === 'invalid release requirement evidence') throw error;
    throw Error('invalid release requirement evidence', { cause: error });
  }
  if (options.requireComplete && !result.complete) throw Error('incomplete release requirements');
  return result;
}
