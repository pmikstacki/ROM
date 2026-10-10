// Check retained HTTP assertions; execution/source admission remains a separate boundary.
import { isDeepStrictEqual } from 'node:util';

const storeLimit = 'single-process synchronous file CAS with file/directory fsync; no power-loss certification';
const fail = () => { throw Error('invalid HTTP recovery evidence'); };
function receipt(value) {
  if (!value || typeof value.expected_identity !== 'string' || !value.expected_identity.length ||
      value.expected_identity.length > 4096 || value.receipt?.identity !== value.expected_identity) fail();
}
function state(value, revision, counts, deleted) {
  if (!value || value.row?.revision !== revision || value.events_for_id !== revision ||
      !isDeepStrictEqual(value.counts, counts) ||
      (deleted ? value.row.value !== null : !value.row.value || value.row.value.title !== 'A')) fail();
  receipt(value);
}

export function requireRecoveryHttpReport(report, adapter) {
  if (!['sqlite', 'redb'].includes(adapter) || report?.adapter !== adapter ||
      report.exact_first_and_replay !== true || report.process_restart !== true ||
      report.pending_store_limit !== storeLimit ||
      !isDeepStrictEqual(Object.keys(report.counts ?? {}).sort(),
        ['committed', 'delete_replayed', 'deleted', 'replayed', 'retired'])) fail();
  const { committed, replayed, deleted, delete_replayed: deleteReplayed, retired } = report.counts;
  for (const value of [committed, replayed]) state(value, 2, [1, 2, 2, 1], false);
  for (const value of [deleted, deleteReplayed]) state(value, 3, [1, 3, 3, 1], true);
  if (retired?.row?.revision !== 3 || committed.expected_identity !== replayed.expected_identity ||
      deleted.expected_identity !== deleteReplayed.expected_identity ||
      typeof committed.wire !== 'string' || !committed.wire.includes('9007199254740993')) fail();
  const bodies = report.trace?.bodies;
  if (!Array.isArray(bodies) || bodies.length < 3 || bodies.length > 128 ||
      bodies.some(body => typeof body !== 'string' || Buffer.byteLength(body) > 1024 * 1024) ||
      bodies[1] !== bodies[2] || !bodies[1].includes('9007199254740993') || !/"expected":1(?:,|})/.test(bodies[1])) fail();
  try { for (const body of bodies) JSON.parse(body); } catch { fail(); }
  return { adapter, exact_first_and_replay: true, process_restart: true, pending_store_limit: storeLimit };
}
