import { decodeRecord, encodeRecord } from "./record.ts";
import { createRecordStorage } from "./record-storage.ts";
import type { MutationRecoveryOptions } from "./types.ts";
/** Serialized compare/exchange ownership; a record is published only after host durability acknowledgement. */
export function intentStorage(
  options: MutationRecoveryOptions,
  maxBytes: number,
) {
  return createRecordStorage({
    store: options.store,
    slot: options.slot,
    newVersion: options.newVersion,
    decode: (payload) => decodeRecord(payload, maxBytes),
    encode: (record) => encodeRecord(record, maxBytes),
  });
}
