import { randomUUID } from "node:crypto";
import { mkdtempSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createClient } from "../../src/lib/client/client.ts";
import type { PendingIntentStore, RecoveryBinding, StoredIntent } from "../../src/recovery.ts";

/** Real file slot with synchronous comparison: atomic for concurrent helpers in this test process. */
export function fileStore(): PendingIntentStore & { path: string; contents(): string } {
  const path = join(mkdtempSync(join(tmpdir(), "rom-recovery-unit-")), "intent.json");
  function read(): StoredIntent | null { return existsSync(path) ? JSON.parse(readFileSync(path, "utf8")) : null; }
  return {
    path, contents: () => existsSync(path) ? readFileSync(path, "utf8") : "",
    async read() { return read(); },
    async compareExchange(_slot, expected, next) {
      if ((read()?.version ?? null) !== expected) return false;
      writeFileSync(path, JSON.stringify(next));
      return true;
    },
  };
}
export const principal = { authority: "fixture", kind: "human", subject: "alice" } as const;
export const operation = { type: "replace", input: { title: "A", count: 9007199254740993n } } as const;
export const identity = { expected: 7n, idempotency: "save-A", retryEpoch: 0n };
export const ready = () => new Response('{"key":{"kind":"notes","id":"one"},"revision":8,"value":{"title":"A","count":9007199254740993}}', { headers: { "content-type": "application/json" } });
export function binding(fetch: typeof globalThis.fetch): RecoveryBinding { return { principal, client: createClient({ base: "http://fixture/api", fetch }) }; }
export function options(store: PendingIntentStore, current: RecoveryBinding) {
  return { namespace: "fixture-host", slot: "notes-editor", target: { kind: "notes", id: "one" }, binding: current, store, newVersion: randomUUID };
}
