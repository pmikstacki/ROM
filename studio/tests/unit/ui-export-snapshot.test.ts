import test from "node:test";
import assert from "node:assert/strict";
import {
  captureExportSnapshot,
  type ExportIdentity,
} from "../../src/lib/ui/export-snapshot.ts";
import { parseWire, stringifyWire } from "../../src/lib/client/codec.ts";
import type { WireValue } from "../../src/lib/client/types.ts";
const identity = (): ExportIdentity => ({
  principal: { authority: "local", kind: "human", subject: "alice" },
  authorityTicket: "session-1",
  resource: {
    kind: "document",
    id: '["local","human","alice"]',
    revision: 9007199254740993n,
  },
  selectedDate: "2026-10-07",
  format: "pdf",
  locale: "pl-PL",
});
const wireClone = (v: WireValue) => parseWire(stringifyWire(v, 16384), 16384);

test("capture detaches exact context and every export attempt receives fresh data", () => {
  const context = identity();
  const input = { title: "original", count: 9007199254740993n };
  const captured = captureExportSnapshot({
    identity: context,
    value: input,
    clone: (v) => ({ ...v }),
  });
  context.resource.id = "later";
  context.locale = "en-US";
  input.title = "later";
  const renderer = captured.value;
  renderer.title = "renderer mutation";
  assert.equal(captured.value.title, "original");
  assert.equal(captured.identity.resource.id, '["local","human","alice"]');
  assert.equal(captured.identity.resource.revision, 9007199254740993n);
  assert.equal(captured.identity.locale, "pl-PL");
});

test("wire clone preserves integer precision and member numeric token categories", () => {
  const input = parseWire(
    '{"integer":9007199254740993,"float":1.0,"exponent":1e0}',
    16384,
  );
  const captured = captureExportSnapshot({
    identity: identity(),
    value: input,
    clone: wireClone,
  });
  assert.equal(
    stringifyWire(captured.value, 16384),
    stringifyWire(input, 16384),
  );
});

test("captured identity is deeply immutable and excludes extra fields", () => {
  const captured = captureExportSnapshot({
    identity: { ...identity(), credential: "must not copy" },
    value: "content",
    clone: (v) => v,
  });
  assert.equal(Object.hasOwn(captured.identity, "credential"), false);
  assert.throws(() => {
    captured.identity.resource.id = "new";
  });
  assert.throws(() => {
    captured.identity.principal!.subject = "bob";
  });
});

test("identity is captured before the clone callback can change host selection", () => {
  const context = identity();
  const captured = captureExportSnapshot({
    identity: context,
    value: "content",
    clone: (v) => {
      context.authorityTicket = "new";
      context.resource.id = "new";
      return v;
    },
  });
  assert.equal(captured.identity.authorityTicket, "session-1");
  assert.equal(captured.identity.resource.id, '["local","human","alice"]');
});

test("a snapshot is context, not current authorization or a file operation", () => {
  const context = identity();
  const captured = captureExportSnapshot({
    identity: context,
    value: "content",
    clone: (v) => v,
  });
  context.principal = null;
  assert.equal(captured.identity.principal?.subject, "alice");
  assert.deepEqual(Object.keys(captured).sort(), ["identity", "value"]);
});
