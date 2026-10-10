import { createMutationRecovery } from "./controller.ts";
import { parseWire } from "../client/codec.ts";
import {
  decodeRecord,
  identifier,
  invocationValue,
  principalKey,
  principalValue,
} from "./record.ts";
import type { Operation } from "../client/types.ts";
import type { MutationRecoveryOptions } from "./types.ts";

export interface RestoreMutationRecoveryOptions extends Omit<
  MutationRecoveryOptions,
  "target"
> {
  kind: string;
  /** Restrict the stable slot to a specific operation role, such as creation. */
  operation?: Operation["type"];
}

/** Discover a persisted target without dispatch. Stored bytes never supply server authority. */
export async function restoreMutationRecovery(
  options: RestoreMutationRecoveryOptions,
) {
  const namespace = identifier(options.namespace),
    slot = identifier(options.slot),
    kind = identifier(options.kind);
  const store = options.store,
    operation = options.operation;
  if (
    operation &&
    !["create", "replace", "patch", "delete", "action"].includes(operation)
  )
    throw Error("invalid recovery operation scope");
  const maxBytes = options.maxBytes ?? 1048576;
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 1)
    throw Error("invalid recovery byte limit");
  const principal = options.binding.principal
    ? principalValue(options.binding.principal)
    : null;
  if (!principal) return null;
  const owner = principalKey(principal),
    client = options.binding.client,
    generation = client.generation;
  let discovering = true;
  function current() {
    if (
      options.binding.client !== client ||
      client.generation !== generation ||
      !options.binding.principal ||
      principalKey(options.binding.principal) !== owner
    )
      throw Error("recovery binding changed");
  }
  function checked(payload: string) {
    const record = decodeRecord(payload, maxBytes);
    if (record.namespace !== namespace || record.target.kind !== kind)
      throw Error("recovery scope mismatch");
    if (principalKey(record.principal) !== owner)
      throw Error("recovery owner mismatch");
    if (
      operation &&
      ((record.accepted &&
        invocationValue(record.accepted.wire, maxBytes).operation.type !==
          operation) ||
        (record.draftWire &&
          (parseWire(record.draftWire, maxBytes) as Operation).type !==
            operation))
    )
      throw Error("recovery operation mismatch");
    return record;
  }
  async function read() {
    if (discovering) current();
    const stored = await store.read(slot);
    if (discovering) current();
    if (!stored) return null;
    identifier(stored.version);
    return { stored, record: checked(stored.payload) };
  }
  const found = await read();
  if (!found) return null;
  const target = { ...found.record.target };
  const recovery = createMutationRecovery({
    ...options,
    namespace,
    slot,
    target,
    maxBytes,
    binding: { client, principal },
    store: {
      async read() {
        return (await read())?.stored ?? null;
      },
      compareExchange(key, expected, next) {
        if (next) checked(next.payload);
        return store.compareExchange(key, expected, next);
      },
    },
  });
  try {
    await recovery.restore();
    current();
    discovering = false;
    return { target, recovery };
  } catch (problem) {
    recovery.dispose();
    throw problem;
  }
}
