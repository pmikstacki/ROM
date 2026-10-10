import { test } from "node:test";
import assert from "node:assert/strict";
import { createClient } from "../../src/client.ts";
import { createMutationRecovery } from "../../src/recovery.ts";
import { fileStore, binding, options, operation, identity, ready } from "./mutation-recovery-support.test.ts";

test("lost acknowledgement restores exact accepted wire into a new client without creating an identity", async () => {
  const store = fileStore(), bodies: string[] = [];
  const first = createMutationRecovery(options(store, binding(async (_url, init) => { bodies.push(String(init?.body)); throw new Error("connection lost"); })));
  await first.stage(operation);
  await first.begin(identity);
  await assert.rejects(first.retry());
  assert.equal(first.state.phase, "unknown");
  assert.equal(first.state.hasUnresolvedIntent, true);
  const restored = createMutationRecovery(options(store, binding(async (_url, init) => { bodies.push(String(init?.body)); return ready(); })));
  await restored.restore();
  assert.equal(restored.state.phase, "unknown");
  await restored.retry();
  assert.equal(bodies.length, 2);
  assert.equal(bodies[0], bodies[1]);
  assert.match(bodies[1], /"count":9007199254740993/);
  assert.match(bodies[1], /"expected":7/);
  assert.match(bodies[1], /"idempotency":"save-A"/);
  assert.equal(restored.state.commitKnowledge, "committed");
  assert.equal(restored.state.hasDraft, false);
  assert.doesNotMatch(store.contents(), /"result"|csrf|generation|cookie|bearer/i);
});

test("later B and C drafts survive reload and A confirmation and never auto-dispatch", async () => {
  const store = fileStore(), bodies: string[] = [];
  let lose = true;
  const lane = createMutationRecovery(options(store, binding(async (_url, init) => { bodies.push(String(init?.body)); if (lose) throw Error("lost"); return ready(); })));
  await lane.stage(operation); await lane.begin(identity); await assert.rejects(lane.retry());
  await lane.stage({ type: "replace", input: { title: "B" } });
  await lane.stage({ type: "replace", input: { title: "C" } });
  await assert.rejects(lane.begin({ expected: 8n, idempotency: "unsafe-new-key" }));
  lose = false;
  const restored = createMutationRecovery(options(store, binding(async (_url, init) => { bodies.push(String(init?.body)); return ready(); })));
  await restored.restore(); await restored.retry();
  assert.equal(bodies.length, 2); assert.equal(bodies[0], bodies[1]);
  assert.equal(restored.state.hasDraft, true);
  const remaining = restored.state.draft;
  assert.equal(remaining?.type, "replace");
  assert.equal(remaining?.type === "replace" ? remaining.input.title : null, "C");
  await restored.begin({ expected: 8n, idempotency: "save-C" });
  assert.equal(bodies.length, 2);
  assert.match(store.contents(), /save-C/);
  await restored.retry();
  assert.match(bodies[2], /"expected":8/); assert.match(bodies[2], /"idempotency":"save-C"/);
  assert.match(bodies[2], /"title":"C"/);
});

test("snapshots, subscribed values and result promises cannot mutate controller-owned values", async () => {
  const lane = createMutationRecovery(options(fileStore(), binding(async () => ready())));
  await lane.stage(operation);
  const snapshot = lane.state;
  if (snapshot.draft?.type === "replace") snapshot.draft.input.title = "tampered";
  lane.subscribe(s => { if (s.draft?.type === "replace") s.draft.input.title = "listener mutation"; });
  assert.equal(lane.state.draft?.type, "replace");
  await lane.begin(identity); const result = await lane.retry();
  result.value = null;
  assert.notEqual(lane.state.result?.value, null);
});

test("storage refusal before dispatch and after acknowledgement prevents replacement identity", async () => {
  const backing = fileStore(); let writes = 0, failAt = 1, calls = 0;
  const store = { read: backing.read, async compareExchange(...args: Parameters<typeof backing.compareExchange>) { writes++; if (writes === failAt) throw Error("private store error"); return backing.compareExchange(...args); } };
  const lane = createMutationRecovery(options(store, binding(async () => { calls++; return ready(); })));
  await assert.rejects(lane.stage(operation)); assert.equal(calls, 0); assert.equal(lane.state.phase, "storage_error");
  failAt = 100; await lane.restore(); await lane.stage(operation); await lane.begin(identity);
  failAt = writes + 2; await assert.rejects(lane.retry());
  assert.equal(calls, 1); assert.equal(lane.state.phase, "storage_error");
  await assert.rejects(lane.begin({ expected: 8n, idempotency: "replacement" }));
  assert.match(backing.contents(), /save-A/);
});

for (const category of ["denied", "conflict", "identity_expired", "identity_mismatch", "history_gap", "not_committed"]) {
  test(`a ${category} retry cannot erase uncertainty of an earlier attempt`, async () => {
    let first = true;
    const lane = createMutationRecovery(options(fileStore(), binding(async () => {
      if (first) { first = false; throw Error("lost"); }
      return new Response(JSON.stringify({ error: category }), { status: category === "not_committed" ? 503 : category === "conflict" ? 409 : 403, headers: { "content-type": "application/json" } });
    })));
    await lane.stage(operation); await lane.begin(identity); await assert.rejects(lane.retry()); await assert.rejects(lane.retry());
    assert.equal(lane.state.commitKnowledge, "unknown"); assert.equal(lane.state.hasUnresolvedIntent, true);
    await assert.rejects(lane.discard({ acknowledgePossibleCommit: false }));
    await lane.discard({ acknowledgePossibleCommit: true }); assert.equal(lane.state.phase, "idle");
  });
}

test("not_committed on a first attempt establishes failure without an earlier uncertain history", async () => {
  const lane = createMutationRecovery(options(fileStore(), binding(async () => new Response('{"error":"not_committed"}', { status: 503, headers: { "content-type": "application/json" } }))));
  await lane.stage(operation); await lane.begin(identity); await assert.rejects(lane.retry());
  assert.equal(lane.state.commitKnowledge, "not_committed"); assert.equal(lane.state.hasUnresolvedIntent, false);
});

test("principal switch and logout quarantine private draft and stale response promise", async () => {
  let resolve!: (response: Response) => void;
  const store = fileStore();
  const lane = createMutationRecovery(options(store, binding(async () => new Promise(r => { resolve = r; }))));
  await lane.stage(operation); await lane.begin(identity);
  const pending = lane.retry(); await new Promise(r => setImmediate(r));
  const hidden: unknown[] = [];
  lane.subscribe(s => { if (s.phase === "quarantined") hidden.push([s.draft, s.result, s.error]); });
  await lane.rebind({ ...binding(async () => ready()), principal: null });
  resolve(ready()); await assert.rejects(pending, /binding changed/);
  assert.equal(lane.state.phase, "quarantined"); assert.deepEqual(hidden.at(-1), [null, null, null]);
  assert.match(store.contents(), /save-A/);
  await lane.rebind({ ...binding(async () => ready()), principal: { authority: "fixture", kind: "human", subject: "bob" } });
  await lane.restore(); assert.equal(lane.state.phase, "quarantined"); await assert.rejects(lane.retry());
});

test("same principal invalidation is observed without retaining old-generation disclosures", async () => {
  let resolve!: (response: Response) => void;
  const current = binding(async () => new Promise(r => { resolve = r; }));
  const lane = createMutationRecovery(options(fileStore(), current));
  await lane.stage(operation); await lane.begin(identity);
  const pending = lane.retry(); await new Promise(r => setImmediate(r));
  current.client.invalidateSession(); resolve(ready()); await assert.rejects(pending);
  assert.equal(lane.state.phase, "unknown");
  assert.equal(lane.state.result, null); assert.equal(lane.state.hasUnresolvedIntent, true);
});

test("principal-null restore performs no host storage read", async () => {
  let reads = 0;
  const lane = createMutationRecovery(options({ async read() { reads++; throw Error("no private read"); }, async compareExchange() { throw Error("no write"); } }, { ...binding(async () => ready()), principal: null }));
  await lane.restore(); assert.equal(reads, 0); assert.equal(lane.state.draft, null); assert.equal(lane.state.error, null);
});

test("CAS cannot delete another helper's newer record", async () => {
  const store = fileStore(), first = createMutationRecovery(options(store, binding(async () => ready())));
  await first.stage(operation);
  const second = createMutationRecovery(options(store, binding(async () => ready()))); await second.restore();
  await second.stage({ type: "replace", input: { title: "newer" } });
  await assert.rejects(first.discard({ acknowledgePossibleCommit: true }));
  assert.match(store.contents(), /newer/); assert.equal(first.state.phase, "storage_error");
});

test("already aborted dispatch retains prepared state, while disposal retains attempted storage", async () => {
  const store = fileStore(); let calls = 0;
  const lane = createMutationRecovery(options(store, binding(async () => { calls++; throw Error("lost"); })));
  await lane.stage(operation); await lane.begin(identity);
  await assert.rejects(lane.retry(AbortSignal.abort()));
  assert.equal(calls, 0); assert.equal(lane.state.phase, "prepared");
  await assert.rejects(lane.retry()); lane.dispose(); assert.equal(lane.state.draft, null);
  assert.match(store.contents(), /save-A/);
});

test("same-principal rebind retries exact identity using fresh transport credentials", async () => {
  const store = fileStore(), bodies: string[] = [], tokens: string[] = [];
  const make = (token: string, lost: boolean) => ({ principal: { authority: "fixture", kind: "human" as const, subject: "alice" }, client: createClient({ base: "http://fixture/api", csrf: () => token, fetch: async (_url, init) => { bodies.push(String(init?.body)); tokens.push(new Headers(init?.headers).get("x-rom-csrf")!); if (lost) throw Error("lost"); return ready(); } }) });
  const lane = createMutationRecovery(options(store, make("fixture-old", true)));
  await lane.stage(operation); await lane.begin(identity); await assert.rejects(lane.retry());
  await lane.rebind(make("fixture-new", false)); await lane.retry();
  assert.equal(bodies[0], bodies[1]); assert.deepEqual(tokens, ["fixture-old", "fixture-new"]);
  assert.doesNotMatch(store.contents(), /fixture-old|fixture-new|csrf|generation/);
});

for (const other of [
  { authority: "other", kind: "human" as const, subject: "alice" },
  { authority: "fixture", kind: "service" as const, subject: "alice" },
  { authority: "fixture", kind: "human" as const, subject: "bob" },
]) {
  test(`changing durable principal ${other.authority}/${other.kind}/${other.subject} never dispatches owner intent`, async () => {
    const store = fileStore(); let calls = 0;
    const lane = createMutationRecovery(options(store, binding(async () => { calls++; return ready(); })));
    await lane.stage(operation); await lane.begin(identity);
    await lane.rebind({ ...binding(async () => { calls++; return ready(); }), principal: other });
    assert.equal(lane.state.phase, "quarantined"); assert.equal(lane.state.draft, null); await assert.rejects(lane.retry());
    assert.equal(calls, 0); assert.match(store.contents(), /save-A/);
  });
}

for (const failAt of [2, 3]) {
  test(`durability refusal at write ${failAt} prevents invoke dispatch`, async () => {
    const backing = fileStore(); let writes = 0, calls = 0;
    const store = { read: backing.read, async compareExchange(...args: Parameters<typeof backing.compareExchange>) { writes++; if (writes === failAt) return false; return backing.compareExchange(...args); } };
    const lane = createMutationRecovery(options(store, binding(async () => { calls++; return ready(); })));
    await lane.stage(operation);
    if (failAt === 2) await assert.rejects(lane.begin(identity));
    else { await lane.begin(identity); await assert.rejects(lane.retry()); }
    assert.equal(calls, 0); assert.equal(lane.state.phase, "storage_error");
  });
}

test("reload cannot replace a live accepted attempt", async () => {
  let resolve!: (response: Response) => void;
  const lane = createMutationRecovery(options(fileStore(), binding(async () => new Promise(r => { resolve = r; }))));
  await lane.stage(operation); await lane.begin(identity);
  const pending = lane.retry(); await new Promise(r => setImmediate(r));
  try { await assert.rejects(lane.restore(), /pending/); }
  finally { resolve(ready()); await pending; }
});

test("principal switch while persistence is awaiting never publishes a late private draft", async () => {
  const backing = fileStore(); let finish!: () => void;
  const store = { read: backing.read, async compareExchange(...args: Parameters<typeof backing.compareExchange>) { await new Promise<void>(r => { finish = r; }); return backing.compareExchange(...args); } };
  const lane = createMutationRecovery(options(store, binding(async () => ready())));
  const staging = lane.stage(operation); await new Promise(r => setImmediate(r));
  await lane.rebind({ ...binding(async () => ready()), principal: null }); finish();
  await assert.rejects(staging, /binding changed/); assert.equal(lane.state.draft, null); assert.equal(lane.state.error, null);
});

test("principal switch during refusal notification prevents old error disclosure through retry promise", async () => {
  const lane = createMutationRecovery(options(fileStore(), binding(async () => new Response('{"error":"denied"}', { status: 403, headers: { "content-type": "application/json" } }))));
  await lane.stage(operation); await lane.begin(identity);
  lane.subscribe(state => {
    if (state.phase === "rejected") void lane.rebind({ ...binding(async () => ready()), principal: null });
  });
  await assert.rejects(lane.retry(), /binding changed/);
  assert.equal(lane.state.error, null); assert.equal(lane.state.draft, null);
});

test("switching away and back cannot revive an old principal's delayed completion", async () => {
  let resolve!: (response: Response) => void;
  const owner = binding(async () => new Promise(r => { resolve = r; }));
  const lane = createMutationRecovery(options(fileStore(), owner));
  await lane.stage(operation); await lane.begin(identity);
  const pending = lane.retry(); await new Promise(r => setImmediate(r));
  await lane.rebind({ ...binding(async () => ready()), principal: null });
  await lane.rebind(binding(async () => ready()));
  resolve(ready()); await assert.rejects(pending, /binding changed/);
  assert.equal(lane.state.phase, "quarantined"); assert.equal(lane.state.result, null);
  await lane.restore(); assert.equal(lane.state.phase, "unknown");
});

test("caller cancellation after dispatch retains unknown intent without a server cancellation request", async () => {
  let requests = 0, resolve!: (response: Response) => void;
  const lane = createMutationRecovery(options(fileStore(), binding(async () => { requests++; return new Promise(r => { resolve = r; }); })));
  await lane.stage(operation); await lane.begin(identity);
  const abort = new AbortController(), pending = lane.retry(abort.signal);
  await new Promise(r => setImmediate(r)); abort.abort(); resolve(ready());
  await assert.rejects(pending); assert.equal(requests, 1);
  assert.equal(lane.state.commitKnowledge, "unknown"); assert.equal(lane.state.hasUnresolvedIntent, true);
});
