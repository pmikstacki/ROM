import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  parseStudioBootstrap,
  createStudioBootstrap,
} from "../../src/lib/application/bootstrap.ts";
import type { IndexedDbIntentStore } from "../../src/lib/recovery/indexeddb-store.ts";

const config = () => ({
  version: 1,
  authority: "deployment-authority",
  recovery: {
    namespace: "application-one",
    retryEpoch: "18446744073709551615",
    maxBytes: 16384,
    intentStore: {
      name: "explicit-intents",
      maxBytes: 20480,
      maxSlots: 64,
      timeoutMs: 2000,
    },
    editorStore: {
      name: "explicit-editors",
      maxBytes: 20480,
      maxSlots: 64,
      timeoutMs: 2000,
    },
  },
});
const principal = {
  authority: "deployment-authority",
  kind: "human" as const,
  subject: "alice",
};
const target = { kind: "notes", id: "one" };

test("creation slots preserve owner and kind without colliding with Resource IDs", async () => {
  const instance = await createStudioBootstrap(config(), environment());
  const slot = instance.profile.recovery.creationSlot!(
    principal,
    "notes",
    "intent",
  );
  assert.deepEqual(JSON.parse(slot), [
    "application-one",
    "intent",
    "deployment-authority",
    "human",
    "alice",
    "notes",
  ]);
  for (const id of ["new", "", "creation", slot, '["notes","new"]']) {
    if (!id) continue;
    assert.notEqual(
      slot,
      instance.profile.recovery.slot(
        principal,
        { kind: "notes", id },
        "intent",
      ),
    );
  }
  assert.notEqual(
    slot,
    instance.profile.recovery.creationSlot!(principal, "notes", "editor"),
  );
  assert.notEqual(
    slot,
    instance.profile.recovery.creationSlot!(
      { ...principal, subject: "bob" },
      "notes",
      "intent",
    ),
  );
  assert.notEqual(
    slot,
    instance.profile.recovery.creationSlot!(principal, "other", "intent"),
  );
  instance.close();
});

test("creation slots reject foreign authority and invalid or oversized scope", async () => {
  const instance = await createStudioBootstrap(config(), environment());
  assert.throws(
    () =>
      instance.profile.recovery.creationSlot!(
        { ...principal, authority: "foreign" },
        "notes",
        "intent",
      ),
    /bootstrap/,
  );
  for (const kind of ["", "bad\u0085kind", "bad\ud800", "x".repeat(4096)])
    assert.throws(
      () => instance.profile.recovery.creationSlot!(principal, kind, "editor"),
      /bootstrap/,
    );
  instance.close();
});

test("the frontend accepts the shared backend bootstrap fixture without epoch conversion", () => {
  const data = readFileSync(
    new URL("../fixtures/studio-bootstrap.json", import.meta.url),
    "utf8",
  );
  assert.deepEqual(parseStudioBootstrap(data), config());
});
test("bootstrap identifiers accept Unicode scalars and reject control or lone surrogate values like the Rust host", () => {
  for (const field of [
    "authority",
    "namespace",
    "intentStore",
    "editorStore",
  ] as const) {
    for (const invalid of [
      "bad\u0085id",
      "bad\u009fid",
      "bad\ud800id",
      "bad\udfffid",
    ]) {
      const value = config();
      if (field === "authority") value.authority = invalid;
      else if (field === "namespace") value.recovery.namespace = invalid;
      else value.recovery[field].name = invalid;
      assert.throws(
        () => parseStudioBootstrap(JSON.stringify(value)),
        /bootstrap/,
        `${field}: ${JSON.stringify(invalid)}`,
      );
    }
  }
  const value = config();
  value.authority = "deployment-zażółć-🌍";
  assert.equal(
    parseStudioBootstrap(JSON.stringify(value)).authority,
    value.authority,
  );
});
function environment(failSecond = false) {
  const opened: string[] = [],
    closed: string[] = [];
  return {
    opened,
    closed,
    now: () => 100,
    uuid: () => "host-generated-key",
    async openStore(options: { name: string }): Promise<IndexedDbIntentStore> {
      opened.push(options.name);
      if (failSecond && opened.length === 2) throw Error("storage unavailable");
      return {
        async read() {
          return null;
        },
        async compareExchange() {
          return true;
        },
        close() {
          closed.push(options.name);
        },
      };
    },
  };
}

test("host bootstrap preserves exact retry epoch and explicit stores without credential inference", async () => {
  const env = environment(),
    input = parseStudioBootstrap(JSON.stringify(config()));
  const instance = await createStudioBootstrap(input, env);
  assert.deepEqual(env.opened, ["explicit-intents", "explicit-editors"]);
  assert.equal(instance.profile.recovery.retryEpoch(), 18446744073709551615n);
  assert.equal(instance.profile.now(), 100);
  assert.equal(instance.profile.recovery.newCommandKey(), "host-generated-key");
  const slot = instance.profile.recovery.slot(principal, target, "intent");
  assert.deepEqual(JSON.parse(slot), [
    "application-one",
    "intent",
    "deployment-authority",
    "human",
    "alice",
    "notes",
    "one",
  ]);
  assert.notEqual(
    slot,
    instance.profile.recovery.slot(principal, target, "editor"),
  );
  instance.close();
  instance.close();
  assert.deepEqual(env.closed, ["explicit-intents", "explicit-editors"]);
});

test("bootstrap rejects absent identity, inferred configuration, ambiguous epochs and unbounded storage", () => {
  for (const mutate of [
    (c: ReturnType<typeof config>) => {
      c.authority = "";
    },
    (c) => {
      c.recovery.retryEpoch = "01";
    },
    (c) => {
      c.recovery.retryEpoch = "18446744073709551616";
    },
    (c) => {
      c.recovery.intentStore.maxBytes = c.recovery.maxBytes;
    },
    (c) => {
      c.recovery.editorStore.maxSlots = 0;
    },
    (c) => {
      c.recovery.editorStore.timeoutMs = 60001;
    },
  ] satisfies Array<(c: ReturnType<typeof config>) => void>) {
    const c = config();
    mutate(c);
    assert.throws(() => parseStudioBootstrap(JSON.stringify(c)), /bootstrap/);
  }
  assert.throws(
    () =>
      parseStudioBootstrap(
        JSON.stringify({ ...config(), csrf: "not-configuration" }),
      ),
    /bootstrap/,
  );
  assert.throws(() => parseStudioBootstrap(" ".repeat(65537)), /bootstrap/);
});

test("second store refusal closes the first store and publishes no usable profile", async () => {
  const env = environment(true);
  await assert.rejects(
    createStudioBootstrap(parseStudioBootstrap(JSON.stringify(config())), env),
    /storage unavailable/,
  );
  assert.deepEqual(env.closed, ["explicit-intents"]);
});

test("slots reject another authority and excessive encoded identifiers", async () => {
  const instance = await createStudioBootstrap(
    parseStudioBootstrap(JSON.stringify(config())),
    environment(),
  );
  assert.throws(
    () =>
      instance.profile.recovery.slot(
        { ...principal, authority: "other" },
        target,
        "intent",
      ),
    /bootstrap/,
  );
  assert.throws(
    () =>
      instance.profile.recovery.slot(
        principal,
        { kind: "notes", id: "x".repeat(4096) },
        "intent",
      ),
    /bootstrap/,
  );
  instance.close();
});

test("opened profiles retain a snapshot of host configuration and separate colliding-looking identifiers", async () => {
  const input = parseStudioBootstrap(JSON.stringify(config()));
  const instance = await createStudioBootstrap(input, environment());
  input.authority = "changed";
  input.recovery.namespace = "changed";
  input.recovery.retryEpoch = "0";
  assert.equal(instance.profile.authority, "deployment-authority");
  assert.equal(instance.profile.recovery.retryEpoch(), 18446744073709551615n);
  assert.notEqual(
    instance.profile.recovery.slot(
      principal,
      { kind: "a,b", id: "c" },
      "intent",
    ),
    instance.profile.recovery.slot(
      principal,
      { kind: "a", id: "b,c" },
      "intent",
    ),
  );
  assert.throws(
    () =>
      parseStudioBootstrap(
        JSON.stringify({ ...config(), authority: "bad\u0000authority" }),
      ),
    /bootstrap/,
  );
  instance.close();
});
