import { test } from "node:test";
import assert from "node:assert/strict";
import {
  semanticKind,
  normalizeSemantic,
} from "../../src/lib/renderers/semantic-fields.ts";

test("standard codec selection requires exact immutable identity", () => {
  assert.equal(semanticKind({ name: "rom.color", version: 1 }), "color");
  assert.equal(semanticKind({ name: "rom.color", version: 2 }), undefined);
  assert.equal(semanticKind({ name: "demo-color", version: 1 }), undefined);
});
for (const [kind, input, output] of [
  ["date", "2024-02-29", "2024-02-29"],
  ["time", "12:30:01.123400000", "12:30:01.1234"],
  [
    "datetime",
    "2026-10-05T12:01:02.123456789+02:00",
    "2026-10-05T10:01:02.123456789Z",
  ],
  ["datetime", "0001-01-01T00:00:00Z", "0001-01-01T00:00:00.000000000Z"],
  ["color", "#ABCDEF88", "#abcdef88"],
  ["decimal", "-000012.34000", "-12.34"],
  ["decimal", "-000.000", "0"],
  [
    "decimal",
    "9007199254740993.000000000000000001",
    "9007199254740993.000000000000000001",
  ],
  ["email", "Alice@EXAMPLE.COM", "Alice@example.com"],
  ["url", "https://example.com", "https://example.com/"],
  ["multiline", "one\r\ntwo", "one\r\ntwo"],
  [
    "json-document",
    '{"large":9007199254740993,"decimal":0.000000000000000001}',
    '{"large":9007199254740993,"decimal":0.000000000000000001}',
  ],
] as const)
  test(`semantic ${kind} preserves its canonical wire contract: ${input}`, () => {
    assert.equal(normalizeSemantic(kind, input), output);
  });
for (const [kind, input] of [
  ["date", "2025-02-29"],
  ["date", "0000-01-01"],
  ["time", "24:00:00"],
  ["time", "12:00:60"],
  ["datetime", "2026-10-05T10:00:00"],
  ["datetime", "2026-02-30T10:00:00Z"],
  ["datetime", "2026-10-05T10:00:00-00:00"],
  ["datetime", "0001-01-01T00:00:00+01:00"],
  ["color", "red"],
  ["color", "#123"],
  ["decimal", "1e3"],
  ["decimal", "1."],
  ["email", "a..b@example.com"],
  ["email", "x@-example.com"],
  ["url", "javascript:alert(1)"],
  ["url", "https://user:pass@example.com"],
  ["json-document", '{"invalid":}'],
] as const)
  test(`semantic ${kind} rejects ${input}`, () => {
    assert.throws(() => normalizeSemantic(kind, input));
  });
test("unit magnitude remains exact and unit tokens are explicit", () => {
  assert.deepEqual(
    normalizeSemantic("unit-value", { value: "0012.300", unit: "kg" }),
    { value: "12.3", unit: "kg" },
  );
  assert.throws(() =>
    normalizeSemantic("unit-value", { value: "1", unit: "" }),
  );
  assert.throws(() =>
    normalizeSemantic("unit-value", {
      value: "1",
      unit: "kg",
      unexpected: "x",
    }),
  );
});

test("frontend validates the same semantic codec vectors as the backend", async () => {
  const { readFileSync } = await import("node:fs");
  const { stringifyWire } = await import("../../src/lib/client/codec.ts");
  const { parseWire } = await import("../../src/lib/client/serialization.ts");
  const fixture = parseWire(
    readFileSync(
      new URL(
        "../../../crates/rom-fields/tests/fixtures/semantic-codecs-v1.json",
        import.meta.url,
      ),
      "utf8",
    ),
  );
  assert.ok(Array.isArray(fixture));
  for (const entry of fixture) {
    assert.ok(entry && typeof entry === "object" && !Array.isArray(entry));
    assert.equal(typeof entry.name, "string");
    assert.equal(entry.version, 1);
    const kind = semanticKind({ name: String(entry.name), version: 1 });
    assert.ok(kind);
    assert.ok(Array.isArray(entry.accepted));
    assert.ok(Array.isArray(entry.rejected));
    for (const pair of entry.accepted) {
      assert.ok(Array.isArray(pair));
      assert.deepEqual(
        parseWire(stringifyWire(normalizeSemantic(kind, pair[0]))),
        pair[1],
        String(entry.name),
      );
    }
    for (const input of entry.rejected)
      assert.throws(() => normalizeSemantic(kind, input), String(entry.name));
  }
});
