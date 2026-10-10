import test from "node:test";
import assert from "node:assert/strict";
import {
  createCreationDrafts,
  creationDraftMatches,
  type CreationDraft,
} from "../../src/lib/application/creation-drafts.ts";
import { fileStore, principal } from "./mutation-recovery-support.test.ts";
import type {
  DurablePrincipal,
  PendingIntentStore,
} from "../../src/recovery.ts";
import { randomUUID } from "node:crypto";
import { stringifyWire } from "../../src/lib/client/codec.ts";
import type { WireValue } from "../../src/lib/client/types.ts";

const draft = () => ({
  id: "  invalid ID\n",
  form: {
    mode: "resource" as const,
    descriptor: "create-notes-v1",
    baseRevision: null,
    intents: { count: { mode: "value", value: 9007199254740993n } },
    editors: { count: { text: "not-a-number", invalid: "Invalid integer" } },
  },
});
test("creation proof association compares field names without delimiter collisions", () => {
  const candidate: CreationDraft = draft();
  candidate.form.editors = {};
  candidate.form.intents = {
    a: { mode: "value", value: 1n },
    b: { mode: "value", value: 2n },
  };
  assert.equal(
    creationDraftMatches(candidate, candidate.id, {
      type: "create",
      input: { "a\u0000b": 1n },
    }),
    false,
  );
});
function fixture(store: PendingIntentStore = fileStore(), maxBytes = 16384) {
  let owner: DurablePrincipal | null = principal;
  const instance = createCreationDrafts({
    namespace: "fixture",
    kind: "notes",
    descriptor: "create-notes-v1",
    maxBytes,
    principal: () => owner,
    slot: (p) =>
      JSON.stringify(["create", p.authority, p.kind, p.subject, "notes"]),
    store,
    newVersion: randomUUID,
    publish() {},
  });
  return {
    instance,
    store,
    rebind(next: DurablePrincipal | null) {
      owner = next;
      instance.rebind(next);
    },
  };
}
test("creation drafts restore exact invalid ID and field text without converting large integer values", async () => {
  const first = fixture();
  await first.instance.stage(draft());
  const restored = fixture(first.store);
  await restored.instance.restore();
  assert.equal(
    stringifyWire(restored.instance.state.snapshot as unknown as WireValue),
    stringifyWire(draft() as unknown as WireValue),
  );
  assert.equal(
    restored.instance.state.snapshot!.form.intents.count.value,
    9007199254740993n,
  );
  assert.equal(restored.instance.state.status, "saved");
  const copy = restored.instance.state.snapshot!;
  copy.form.editors.count.text = "changed";
  assert.equal(
    restored.instance.state.snapshot!.form.editors.count.text,
    "not-a-number",
  );
});
test("a principal change during a pending CAS never publishes the former owner's draft", async () => {
  const backing = fileStore();
  let owner: DurablePrincipal = principal;
  let finish!: () => void, started!: () => void;
  const held = new Promise<void>((resolve) => {
    finish = resolve;
  });
  const began = new Promise<void>((resolve) => {
    started = resolve;
  });
  const disclosures: unknown[] = [];
  const instance = createCreationDrafts({
    namespace: "fixture",
    kind: "notes",
    descriptor: "create-notes-v1",
    maxBytes: 16384,
    principal: () => owner,
    slot: (p) =>
      JSON.stringify([p.authority, p.kind, p.subject, "create", "notes"]),
    store: {
      read: backing.read,
      async compareExchange(...args) {
        started();
        await held;
        return backing.compareExchange(...args);
      },
    },
    newVersion: randomUUID,
    publish(state) {
      if (owner.subject === "bob") disclosures.push(state.snapshot);
    },
  });
  const pending = instance.stage(draft());
  await began;
  owner = { ...principal, subject: "bob" };
  finish();
  await assert.rejects(pending, /principal changed/);
  assert.equal(instance.state.snapshot, null);
  assert.ok(disclosures.every((value) => value === null));
});
test("a concurrent creation draft cannot overwrite an unobserved stored version", async () => {
  const first = fixture();
  await first.instance.stage(draft());
  const other = fixture(first.store);
  await assert.rejects(
    other.instance.stage({ ...draft(), id: "other" }),
    /conflict/,
  );
  assert.equal(other.instance.state.status, "error");
  assert.throws(() => other.instance.guardNavigation(), /storage/);
  await other.instance.restore();
  assert.equal(other.instance.state.snapshot!.id, draft().id);
});
test("logout hides creation drafts and restore reads no private storage without an owner", async () => {
  let reads = 0;
  const backing = fileStore();
  const f = fixture({
    read(slot) {
      reads++;
      return backing.read(slot);
    },
    compareExchange: backing.compareExchange,
  });
  await f.instance.stage(draft());
  f.rebind(null);
  assert.equal(f.instance.state.snapshot, null);
  await f.instance.restore();
  assert.equal(reads, 0);
  await assert.rejects(f.instance.stage(draft()), /principal/);
});
test("a changed descriptor refuses stored drafts instead of silently applying them", async () => {
  const f = fixture();
  await f.instance.stage(draft());
  const other = createCreationDrafts({
    namespace: "fixture",
    kind: "notes",
    descriptor: "changed",
    maxBytes: 16384,
    principal: () => principal,
    slot: (p) =>
      JSON.stringify(["create", p.authority, p.kind, p.subject, "notes"]),
    store: f.store,
    newVersion: randomUUID,
    publish() {},
  });
  await assert.rejects(other.restore(), /identity/);
  assert.equal(other.state.snapshot, null);
  assert.throws(() => other.guardNavigation(), /storage/);
  await other.discard();
  assert.doesNotThrow(() => other.guardNavigation());
  assert.equal(await f.store.read("ignored"), null);
});
test("explicit local discard refuses another owner's stored creation envelope", async () => {
  const f = fixture();
  await f.instance.stage(draft());
  const before = f.store.contents();
  const other = fixture(f.store);
  other.rebind({ ...principal, subject: "bob" });
  await assert.rejects(other.instance.discard(), /identity/);
  assert.equal(f.store.contents(), before);
});
test("explicit local discard cannot erase a concurrent replacement after its read", async () => {
  const backing = fileStore();
  let signal!: () => void, release!: () => void;
  const entered = new Promise<void>((resolve) => (signal = resolve));
  const gate = new Promise<void>((resolve) => (release = resolve));
  const f = fixture({
    read: async (slot) => {
      const stored = await backing.read(slot);
      signal();
      await gate;
      return stored;
    },
    compareExchange: backing.compareExchange,
  });
  await f.instance.stage(draft());
  const deletion = f.instance.discard();
  await entered;
  const stored = (await backing.read("ignored"))!;
  const replacement = { ...stored, version: randomUUID() };
  assert.equal(
    await backing.compareExchange("ignored", stored.version, replacement),
    true,
  );
  release();
  await assert.rejects(deletion, /conflict/);
  assert.equal((await backing.read("ignored"))!.version, replacement.version);
});
test("owner change during explicit discard read prevents deletion", async () => {
  const backing = fileStore();
  let signal!: () => void, release!: () => void;
  const entered = new Promise<void>((resolve) => (signal = resolve));
  const gate = new Promise<void>((resolve) => (release = resolve));
  const f = fixture({
    read: async (slot) => {
      const stored = await backing.read(slot);
      signal();
      await gate;
      return stored;
    },
    compareExchange: backing.compareExchange,
  });
  await f.instance.stage(draft());
  const before = backing.contents();
  const deletion = f.instance.discard();
  await entered;
  f.rebind({ ...principal, subject: "bob" });
  release();
  await assert.rejects(deletion, /principal/);
  assert.equal(backing.contents(), before);
});
test("a byte-limit refusal preserves the prior creation draft and blocks navigation", async () => {
  const f = fixture(fileStore(), 1024);
  await f.instance.stage(draft());
  await assert.rejects(f.instance.stage({ ...draft(), id: "x".repeat(1024) }));
  assert.equal(f.instance.state.status, "error");
  assert.throws(() => f.instance.guardNavigation(), /storage/);
  await f.instance.restore();
  assert.equal(f.instance.state.snapshot!.id, draft().id);
});

test("save confirmation clears only its matching creation draft and preserves later edits", async () => {
  const f = fixture();
  const original = draft();
  await f.instance.stage(original);
  const later = { ...draft(), id: "later-id" };
  await f.instance.stage(later);
  assert.equal(await f.instance.discard(original), false);
  assert.equal(f.instance.state.snapshot!.id, "later-id");
  assert.equal(await f.instance.discard(later), true);
  assert.equal(f.instance.state.snapshot, null);
  await fixture(f.store).instance.restore();
});
