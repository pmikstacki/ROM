import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createMutationRecovery } from "../../src/lib/recovery/controller.ts";
import { decodeRecord } from "../../src/lib/recovery/record.ts";
import { stringifyWire } from "../../src/lib/client/codec.ts";
import { binding, fileStore, identity, operation, options, principal, ready } from "./mutation-recovery-support.test.ts";
import type { WireValue } from "../../src/lib/client/types.ts";

const proof = "sha256:" + "ab".repeat(32);
const otherProof = "sha256:" + "cd".repeat(32);
const maxBytes = 16384;

test("accepted editor proof is durable before dispatch and exact command survives restart and receipt recovery", async () => {
  const store = fileStore(), bodies: string[] = [];
  let lost = true;
  const current = binding(async (_url, init) => {
    assert.equal(decodeRecord(JSON.parse(store.contents()).payload, maxBytes).accepted?.editorProof, proof);
    bodies.push(String(init?.body));
    if (lost) throw Error("lost acknowledgement");
    return ready();
  });
  const first = createMutationRecovery(options(store, current));
  await first.stage(operation);
  const requested = { ...identity, editorProof: proof };
  await first.begin(requested);
  requested.editorProof = otherProof;
  assert.equal(first.state.acceptedEditorProof, proof);
  await assert.rejects(first.retry());
  assert.equal(first.state.acceptedEditorProof, proof);
  const restored = createMutationRecovery(options(store, current));
  await restored.restore();
  assert.equal(restored.state.acceptedEditorProof, proof);
  assert.equal(bodies.length, 1);
  await restored.stage({ type: "replace", input: { title: "later" } });
  lost = false;
  await restored.retry();
  assert.equal(restored.state.acceptedEditorProof, proof);
  assert.equal(bodies[0], bodies[1]);
  assert.equal(bodies[0], '{"kind":"notes","id":"one","expected":7,"idempotency":"save-A","operation":{"type":"replace","input":{"title":"A","count":9007199254740993}},"retry_epoch":0}');
  assert.equal(JSON.parse(bodies[0]).editorProof, undefined);
  const completed = createMutationRecovery(options(store, current));
  await completed.restore();
  assert.equal(completed.state.acceptedEditorProof, proof);
  await completed.discard({ acknowledgePossibleCommit: true });
  assert.equal(completed.state.acceptedEditorProof, undefined);
});

for (const category of ["denied", "conflict", "not_committed"]) {
  test(`accepted editor proof survives ${category} outcome and restore`, async () => {
    const store = fileStore();
    const current = binding(async () => new Response(JSON.stringify({ error: category }), {
      status: category === "not_committed" ? 503 : category === "conflict" ? 409 : 403,
      headers: { "content-type": "application/json" },
    }));
    const lane = createMutationRecovery(options(store, current));
    await lane.stage(operation);
    await lane.begin({ ...identity, editorProof: proof });
    await assert.rejects(lane.retry());
    assert.equal(lane.state.acceptedEditorProof, proof);
    const restored = createMutationRecovery(options(store, current));
    await restored.restore();
    assert.equal(restored.state.acceptedEditorProof, proof);
  });
}

test("proof validator rejects malformed or unknown metadata and accepts old records without proof", async () => {
  const store = fileStore();
  const lane = createMutationRecovery(options(store, binding(async () => ready())));
  await lane.stage(operation);
  await lane.begin(identity);
  const record = JSON.parse(JSON.parse(store.contents()).payload);
  assert.equal(decodeRecord(JSON.stringify(record), maxBytes).accepted?.editorProof, undefined);
  assert.equal(decodeRecord(JSON.stringify({ ...record, accepted: { ...record.accepted, editorProof: proof } }), maxBytes).accepted?.editorProof, proof);
  for (const editorProof of [null, true, {}, "", "sha256:" + "A".repeat(64), "sha256:" + "0".repeat(63), "sha256:" + "0".repeat(65), "sha512:" + "0".repeat(64), "sha256:" + "0".repeat(64) + "\n"]) {
    assert.throws(() => decodeRecord(JSON.stringify({ ...record, accepted: { ...record.accepted, editorProof } }), maxBytes));
  }
  assert.throws(() => decodeRecord(JSON.stringify({ ...record, accepted: { ...record.accepted, editorProof: proof, futureProof: "unsupported" } }), maxBytes));
  assert.throws(() => decodeRecord(JSON.stringify({ ...record, editorProof: proof }), maxBytes));
});

test("invalid begin proof leaves durable command unaccepted and sends nothing", async () => {
  const store = fileStore(); let requests = 0;
  const lane = createMutationRecovery(options(store, binding(async () => { requests++; return ready(); })));
  await lane.stage(operation);
  const before = store.contents();
  await assert.rejects(lane.begin({ ...identity, editorProof: "malformed" }));
  assert.equal(store.contents(), before);
  assert.equal(lane.state.acceptedEditorProof, undefined);
  assert.equal(requests, 0);
});

test("accepted proof is hidden after logout or foreign-owner rebind and kept for same-owner renewal", async () => {
  const store = fileStore(), current = binding(async () => ready());
  const lane = createMutationRecovery(options(store, current));
  await lane.stage(operation);
  await lane.begin({ ...identity, editorProof: proof });
  await lane.rebind(binding(async () => ready()));
  assert.equal(lane.state.acceptedEditorProof, proof);
  await lane.rebind({ ...current, principal: { ...principal, subject: "bob" } });
  assert.equal(lane.state.acceptedEditorProof, undefined);
  await lane.restore();
  assert.equal(lane.state.acceptedEditorProof, undefined);
  await lane.rebind({ ...current, principal: null });
  assert.equal(lane.state.acceptedEditorProof, undefined);
  lane.dispose();
  assert.equal(lane.state.acceptedEditorProof, undefined);
});

test("owner change during proof CAS cannot publish the former owner's proof", async () => {
  const backing = fileStore();
  let release!: () => void, started!: () => void, hold = false;
  const held = new Promise<void>(resolve => { release = resolve; });
  const began = new Promise<void>(resolve => { started = resolve; });
  const store = {
    read: backing.read,
    async compareExchange(...args: Parameters<typeof backing.compareExchange>) {
      if (hold) { started(); await held; }
      return backing.compareExchange(...args);
    },
  };
  const lane = createMutationRecovery(options(store, binding(async () => ready())));
  await lane.stage(operation);
  hold = true;
  const pending = lane.begin({ ...identity, editorProof: proof });
  await began;
  await lane.rebind({ ...binding(async () => ready()), principal: { ...principal, subject: "bob" } });
  const disclosures: unknown[] = [];
  lane.subscribe(state => disclosures.push(state.acceptedEditorProof));
  release();
  await assert.rejects(pending, /binding changed/);
  assert.ok(disclosures.every(value => value === undefined));
  assert.equal(lane.state.acceptedEditorProof, undefined);
});

test("bounded native fingerprint distinguishes exact integers and raw editor text for equal operations", async () => {
  const module = await import("../../src/recovery.ts");
  assert.equal(typeof module.editorFingerprint, "function");
  const value = { id: "new", form: { intents: { count: { mode: "value", value: 9007199254740993n } }, editors: { count: { text: "9007199254740993" }, title: { text: "é" } } } };
  const fingerprint = module.editorFingerprint;
  const expected = "sha256:" + createHash("sha256").update('{"id":"new","form":{"intents":{"count":{"mode":"value","value":9007199254740993}},"editors":{"count":{"text":"9007199254740993"},"title":{"text":"é"}}}}', "utf8").digest("hex");
  assert.equal(await fingerprint(value, maxBytes), expected);
  assert.notEqual(await fingerprint({ ...value, id: " new " }, maxBytes), expected);
  assert.notEqual(await fingerprint({ ...value, form: { ...value.form, editors: { ...value.form.editors, count: { text: "09007199254740993" } } } }, maxBytes), expected);
  assert.notEqual(await fingerprint({ ...value, form: { ...value.form, intents: { count: { mode: "value", value: 9007199254740992n } } } }, maxBytes), expected);
  const copied = structuredClone(value);
  const pending = fingerprint(copied, maxBytes);
  copied.id = "later";
  assert.equal(await pending, expected);
  for (const limit of [0, -1, 1.5, Number.POSITIVE_INFINITY]) await assert.rejects(fingerprint(value, limit));
  await assert.rejects(fingerprint({ text: "é".repeat(100) }, 150), /byte limit/);
  await assert.rejects(fingerprint({ secret: "x".repeat(maxBytes) }, maxBytes), /byte limit/);
  assert.equal(stringifyWire(value as WireValue).includes("9007199254740993"), true);
});
