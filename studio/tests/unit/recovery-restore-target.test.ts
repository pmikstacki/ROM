import test from "node:test";
import assert from "node:assert/strict";
import {
  createMutationRecovery,
  restoreMutationRecovery,
} from "../../src/recovery.ts";
import {
  fileStore,
  binding,
  options,
  ready,
} from "./mutation-recovery-support.test.ts";
import { parseWire, stringifyWire } from "../../src/lib/client/codec.ts";
import type { WireObject } from "../../src/lib/client/types.ts";

async function prepared() {
  const store = fileStore();
  const original = createMutationRecovery(
    options(
      store,
      binding(async () => {
        throw Error("lost acknowledgement");
      }),
    ),
  );
  await original.stage({
    type: "create",
    input: { title: "exact", count: 9007199254740993n },
  });
  await original.begin({
    expected: null,
    idempotency: "original-create",
    retryEpoch: 18446744073709551615n,
  });
  await assert.rejects(original.retry());
  return store;
}
function restoreOptions(
  store: ReturnType<typeof fileStore>,
  next = binding(async () => ready()),
) {
  const { target: _, ...rest } = options(store, next);
  return { ...rest, kind: "notes", operation: "create" as const };
}
const created = () =>
  new Response(
    '{"key":{"kind":"notes","id":"one"},"revision":1,"value":{"title":"exact","count":9007199254740993}}',
    { headers: { "content-type": "application/json" } },
  );

test("a stable creation slot restores its exact accepted target without dispatch or a replacement key", async () => {
  const store = await prepared();
  const original = JSON.parse(store.contents()).payload;
  let calls = 0;
  const bodies: string[] = [];
  const recovered = await restoreMutationRecovery(
    restoreOptions(
      store,
      binding(async (_url, init) => {
        calls++;
        bodies.push(String(init?.body));
        return created();
      }),
    ),
  );
  assert.ok(recovered);
  assert.deepEqual(recovered.target, { kind: "notes", id: "one" });
  assert.equal(recovered.recovery.state.phase, "unknown");
  assert.equal(calls, 0);
  assert.equal(JSON.parse(store.contents()).payload, original);
  await recovered.recovery.retry();
  assert.equal(calls, 1);
  assert.equal(bodies[0], JSON.parse(original).accepted.wire);
});

test("a restored controller retains normal same-owner rebind and exact retry behavior", async () => {
  const store = await prepared();
  const initial = binding(async () => {
    throw Error("lost");
  });
  const recovered = await restoreMutationRecovery(
    restoreOptions(store, initial),
  );
  assert.ok(recovered);
  initial.client.invalidateSession();
  await recovered.recovery.rebind(binding(async () => created()));
  await recovered.recovery.restore();
  await recovered.recovery.retry();
  assert.equal(recovered.recovery.state.commitKnowledge, "committed");
});

test("a creation recovery scope refuses a later non-create draft before any dispatch", async () => {
  const store = await prepared();
  const before = store.contents();
  const recovered = await restoreMutationRecovery(restoreOptions(store));
  assert.ok(recovered);
  await assert.rejects(
    recovered.recovery.stage({
      type: "replace",
      input: { title: "wrong role" },
    }),
    /storage/,
  );
  assert.equal(store.contents(), before);
});

test("restore with no current principal never reads private storage", async () => {
  let reads = 0;
  const configuration = restoreOptions(fileStore(), {
    ...binding(async () => ready()),
    principal: null,
  });
  configuration.store = {
    async read() {
      reads++;
      throw Error("private read");
    },
    async compareExchange() {
      throw Error("write");
    },
  };
  assert.equal(await restoreMutationRecovery(configuration), null);
  assert.equal(reads, 0);
});

test("restore rejects foreign owner, namespace, kind, and non-create accepted operations", async () => {
  const store = await prepared();
  for (const configure of [
    (value: ReturnType<typeof restoreOptions>) => {
      value.binding = {
        ...value.binding,
        principal: { authority: "fixture", kind: "human", subject: "bob" },
      };
    },
    (value: ReturnType<typeof restoreOptions>) => {
      value.namespace = "foreign";
    },
    (value: ReturnType<typeof restoreOptions>) => {
      value.kind = "other";
    },
    (value: ReturnType<typeof restoreOptions>) => {
      value.operation = "patch" as "create";
    },
  ]) {
    const value = restoreOptions(store);
    configure(value);
    await assert.rejects(
      restoreMutationRecovery(value),
      /scope|owner|operation/,
    );
  }
});

test("a changed stored target between both reads cannot be restored under the first target", async () => {
  const store = await prepared();
  let reads = 0;
  const value = restoreOptions(store);
  value.store = {
    async read(slot) {
      const record = await store.read(slot);
      if (++reads === 2 && record) {
        const payload = parseWire(record.payload) as WireObject;
        (payload.target as WireObject).id = "different";
        const accepted = payload.accepted as WireObject;
        const wire = parseWire(accepted.wire as string) as WireObject;
        wire.id = "different";
        accepted.wire = stringifyWire(wire);
        return { ...record, payload: stringifyWire(payload) };
      }
      return record;
    },
    compareExchange: store.compareExchange,
  };
  await assert.rejects(restoreMutationRecovery(value), /record|scope/);
});

test("generation change during discovery rejects the restored private projection", async () => {
  const store = await prepared();
  const value = restoreOptions(store);
  value.store = {
    async read(slot) {
      const record = await store.read(slot);
      value.binding.client.invalidateSession();
      return record;
    },
    compareExchange: store.compareExchange,
  };
  await assert.rejects(restoreMutationRecovery(value), /binding/);
});
