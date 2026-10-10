import test from "node:test";
import assert from "node:assert/strict";
import {
  captureFormDraft,
  restoreFormDraft,
} from "../../src/lib/resources/form-draft.ts";
import { parseWire, stringifyWire } from "../../src/lib/client/codec.ts";
import type { FieldIntent, WireObject } from "../../src/lib/client/types.ts";
const identity = {
  mode: "resource" as const,
  descriptor: "note-v1",
  baseRevision: 9007199254740993n,
};
test("form snapshot retains invalid raw text independently of prior valid input and large revision", () => {
  const draft = captureFormDraft({
    ...identity,
    intents: { count: { mode: "value", value: 9007199254740993n } },
    editors: { count: { text: "invalid-count", invalid: "Enter an integer." } },
  });
  const restored = restoreFormDraft(draft, identity);
  assert.equal(restored.baseRevision, 9007199254740993n);
  assert.deepEqual(restored.intents.count, {
    mode: "value",
    value: 9007199254740993n,
  });
  assert.equal(restored.editors.count.text, "invalid-count");
});
test("form draft copies preserve lexical decimal categories and isolate snapshots", () => {
  const floats = parseWire(
    '{"one":1.0,"exponent":1e0,"negative":-0.0}',
  ) as WireObject;
  const intents: Record<string, FieldIntent> = {
    values: { mode: "value", value: floats },
  };
  const editors = { values: { state: { values: floats } } };
  const draft = captureFormDraft({ ...identity, intents, editors });
  (intents.values as { mode: "value"; value: WireObject }).value.one = 2;
  const restored = restoreFormDraft(draft, identity);
  assert.equal(
    stringifyWire(
      (restored.intents.values as { mode: "value"; value: WireObject }).value,
    ),
    '{"one":1.0,"exponent":1e0,"negative":-0.0}',
  );
  restored.editors.values.state!.changed = true;
  assert.equal(draft.editors.values.state!.changed, undefined);
});
test("restore rejects changed descriptor and different action or form identity", () => {
  const draft = captureFormDraft({ ...identity, intents: {}, editors: {} });
  assert.throws(
    () => restoreFormDraft(draft, { ...identity, descriptor: "note-v2" }),
    /identity/i,
  );
  assert.throws(
    () => restoreFormDraft(draft, { ...identity, mode: "action" }),
    /identity/i,
  );
});
test("restore retains original base revision rather than adopting a renewed resource revision", () => {
  const draft = captureFormDraft({
    ...identity,
    intents: { title: { mode: "omit" } },
    editors: {},
  });
  assert.equal(
    restoreFormDraft(draft, { ...identity, baseRevision: 9007199254740994n })
      .baseRevision,
    identity.baseRevision,
  );
});
test("restore rejects malformed intent modes and value omissions", () => {
  const draft = captureFormDraft({
    ...identity,
    intents: { title: { mode: "omit" } },
    editors: {},
  });
  draft.intents.title = { mode: "credential" };
  assert.throws(() => restoreFormDraft(draft, identity), /intent/i);
  draft.intents.title = { mode: "value" };
  assert.throws(() => restoreFormDraft(draft, identity), /intent/i);
});
test("snapshot uses an explicit byte bound and excludes functions from persisted editor state", () => {
  assert.throws(
    () =>
      captureFormDraft({
        ...identity,
        intents: {},
        editors: { title: { text: "x".repeat(1024) } },
        maxBytes: 128,
      }),
    /limit|bound|large|bytes/i,
  );
  const editors = { title: { state: { hook: () => {} } } };
  assert.throws(
    () => captureFormDraft({ ...identity, intents: {}, editors }),
    /wire|value|function|unsupported/i,
  );
});

test("form snapshots retain arbitrary own field names without prototype mutation", () => {
  const intents = Object.fromEntries([
    ["__proto__", { mode: "value" as const, value: "field" }],
    ["constructor", { mode: "null" as const }],
  ]);
  const snapshot = captureFormDraft({ ...identity, intents, editors: {} });
  assert.equal(Object.hasOwn(snapshot.intents, "__proto__"), true);
  const restored = restoreFormDraft(snapshot, identity);
  assert.equal(Object.hasOwn(restored.intents, "__proto__"), true);
  assert.deepEqual(restored.intents.__proto__, {
    mode: "value",
    value: "field",
  });
  assert.deepEqual(restored.intents.constructor, { mode: "null" });
  assert.equal(Object.getPrototypeOf(restored.intents), Object.prototype);
});

test("frame set merges simultaneous resource and action drafts without losing either invalid editor", async () => {
  const { mergeFormFrames, readFormFrames } =
    await import("../../src/lib/resources/form-draft.ts");
  const definition = {
    descriptor: "parent-v1",
    resource: "note-v1",
    actions: { save: "save-v1" },
  };
  const resource = captureFormDraft({
    ...identity,
    intents: { count: { mode: "value", value: 9007199254740993n } },
    editors: { count: { text: "invalid-resource", invalid: "integer" } },
  });
  const action = captureFormDraft({
    ...identity,
    mode: "action",
    descriptor: "save-v1",
    intents: { title: { mode: "value", value: "action-C" } },
    editors: { title: { text: "action-C" } },
  });
  let aggregate = mergeFormFrames(
    null,
    { type: "resource" },
    resource,
    definition,
  );
  aggregate = mergeFormFrames(
    aggregate,
    { type: "action", name: "save" },
    action,
    definition,
  );
  const frames = readFormFrames(aggregate, definition);
  assert.equal(frames.resource?.editors.count.text, "invalid-resource");
  assert.equal(frames.actions.save.editors.title.text, "action-C");
  assert.equal(aggregate.baseRevision, identity.baseRevision);
  frames.resource!.editors.count.text = "mutated";
  assert.equal(
    readFormFrames(aggregate, definition).resource?.editors.count.text,
    "invalid-resource",
  );
});
test("frame set rejects wrong action identity, unowned or extra frames and base revision changes", async () => {
  const { mergeFormFrames, readFormFrames } =
    await import("../../src/lib/resources/form-draft.ts");
  const definition = {
    descriptor: "parent-v1",
    resource: "note-v1",
    actions: Object.fromEntries([
      ["__proto__", "proto-v1"],
      ["save", "save-v1"],
    ]),
  };
  const resource = captureFormDraft({ ...identity, intents: {}, editors: {} }),
    aggregate = mergeFormFrames(
      null,
      { type: "resource" },
      resource,
      definition,
    );
  const action = captureFormDraft({
    ...identity,
    mode: "action",
    descriptor: "proto-v1",
    intents: {},
    editors: {},
  });
  const merged = mergeFormFrames(
    aggregate,
    { type: "action", name: "__proto__" },
    action,
    definition,
  );
  assert.equal(
    Object.hasOwn(readFormFrames(merged, definition).actions, "__proto__"),
    true,
  );
  assert.throws(
    () =>
      mergeFormFrames(
        aggregate,
        { type: "action", name: "missing" },
        action,
        definition,
      ),
    /identity|action|frame/i,
  );
  assert.throws(
    () =>
      mergeFormFrames(
        aggregate,
        { type: "action", name: "save" },
        action,
        definition,
      ),
    /identity/i,
  );
  assert.throws(
    () =>
      mergeFormFrames(
        aggregate,
        { type: "resource" },
        { ...resource, baseRevision: 2n },
        definition,
      ),
    /revision/i,
  );
  assert.throws(
    () => readFormFrames(merged, { ...definition, descriptor: "parent-v2" }),
    /identity/i,
  );
  merged.editors.extra = { text: "private" };
  assert.throws(() => readFormFrames(merged, definition), /frame|extra/i);
});
test("aggregate form frame bounds cover all children and preserve exact wire lexical categories", async () => {
  const { mergeFormFrames, readFormFrames } =
    await import("../../src/lib/resources/form-draft.ts");
  const definition = {
    descriptor: "parent-v1",
    resource: "note-v1",
    actions: { save: "save-v1" },
  };
  const value = parseWire(
    '{"one":1.0,"exponent":1e0,"negative":-0.0,"large":9007199254740993}',
  ) as WireObject;
  const leaf = captureFormDraft({
    ...identity,
    intents: { value: { mode: "value", value } },
    editors: { value: { state: { value } } },
  });
  const aggregate = mergeFormFrames(
    null,
    { type: "resource" },
    leaf,
    definition,
  );
  assert.equal(
    stringifyWire(
      readFormFrames(aggregate, definition).resource!.intents.value,
    ),
    '{"mode":"value","value":{"one":1.0,"exponent":1e0,"negative":-0.0,"large":9007199254740993}}',
  );
  assert.throws(
    () =>
      mergeFormFrames(
        aggregate,
        { type: "action", name: "save" },
        captureFormDraft({
          ...identity,
          mode: "action",
          descriptor: "save-v1",
          intents: {},
          editors: { raw: { text: "x".repeat(1024) } },
        }),
        definition,
        { maxBytes: 512, maxFrames: 2, maxChildren: 100, maxDepth: 32 },
      ),
    /bound|bytes|limit|large/i,
  );
  assert.throws(
    () =>
      readFormFrames(aggregate, definition, {
        maxBytes: 65536,
        maxFrames: 1,
        maxChildren: 1,
        maxDepth: 32,
      }),
    /bound|child|limit/i,
  );
});

test("typed small base revision remains bigint through capture and restore", () => {
  const draft = captureFormDraft({
    ...identity,
    baseRevision: 1n,
    intents: {},
    editors: {},
  });
  assert.equal(draft.baseRevision, 1n);
  assert.equal(
    restoreFormDraft(draft, { ...identity, baseRevision: 2n }).baseRevision,
    1n,
  );
});

test("nested small typed frame revisions restore as bigint and decimal revision tokens fail closed", async () => {
  const { mergeFormFrames, readFormFrames } =
    await import("../../src/lib/resources/form-draft.ts");
  const definition = {
    descriptor: "parent-v1",
    resource: "note-v1",
    actions: {},
  };
  const leaf = captureFormDraft({
    ...identity,
    baseRevision: 1n,
    intents: {},
    editors: {},
  });
  const aggregate = mergeFormFrames(
    null,
    { type: "resource" },
    leaf,
    definition,
  );
  assert.equal(aggregate.baseRevision, 1n);
  assert.equal(
    readFormFrames(aggregate, definition).resource?.baseRevision,
    1n,
  );
  const malformed = parseWire(
    stringifyWire(aggregate).replace('"baseRevision":1', '"baseRevision":1.0'),
  );
  assert.throws(
    () => readFormFrames(malformed, definition),
    /revision|integer|float/i,
  );
});
