import { createClient, parseWire, stringifyWire, type WireObject } from "rom-studio/client";
import { createMutationRecovery } from "rom-studio/recovery";
import type { StoredIntent } from "rom-studio/recovery";
import { publicAuthContract } from "./auth-contract.ts";
import { publicStorageContract } from "./storage-contract.ts";

/** Exercise public package resolution, codecs and host-selected intent storage without network effects. */
export async function publicClientContract() {
  await publicAuthContract();
  await publicStorageContract();
  let stored: StoredIntent | null = null;
  let version = 0;
  const recovery = createMutationRecovery({
    namespace: "installed-consumer", slot: "note-editor", target: { kind: "notes", id: "one" },
    binding: { client: createClient({ base: "http://fixture.invalid/api" }), principal: { authority: "fixture", kind: "human", subject: "alice" } },
    store: {
      async read() { return stored; },
      async compareExchange(_slot: string, expected: string | null, next: StoredIntent | null) {
        if ((stored?.version ?? null) !== expected) return false;
        stored = next; return true;
      },
    },
    newVersion: () => String(++version),
  });
  try {
    await recovery.stage({ type: "replace", input: parseWire('{"count":9007199254740993,"ratio":1.0,"open":false,"code":null}') as WireObject });
    await recovery.begin({ expected: 7n, idempotency: "installed-save", retryEpoch: 0n });
    return { phase: recovery.state.phase, unresolved: recovery.state.hasUnresolvedIntent, draft: stringifyWire(recovery.state.draft), persisted: stored !== null };
  } finally { recovery.dispose(); }
}
