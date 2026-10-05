import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";
import { parseWire } from "../../src/lib/client/serialization.ts";
import { enumLabel } from "../../src/lib/client/enum-labels.ts";
import { discovery } from "../../src/lib/client/discovery.ts";
import type { WireValue } from "../../src/lib/client/types.ts";
function fixture(name: string) {
  return parseWire(
    readFileSync(new URL(`../../../${name}`, import.meta.url), "utf8"),
  );
}
function parse(field: WireValue) {
  return discovery({
    version: 1,
    resources: [
      {
        kind: "enum",
        version: 1,
        fields: [field],
        actions: [],
        action_inputs: [],
      },
    ],
  }).resources[0].fields[0];
}
test("shared native discovery retains enum labels through wrappers and action inputs", () => {
  const resource = discovery(
    fixture("examples/consumer/tests/fixtures/enum-labels-discovery-v1.json"),
  ).resources[0];
  assert.deepEqual(resource.fields[0].enum_labels, {
    queued: "Waiting",
    running: "Active",
    paused: "Active",
  });
  const scalar = resource.action_inputs[1].input;
  assert.equal(scalar?.type, "scalar");
  if (scalar?.type === "scalar")
    assert.deepEqual(scalar.value.enum_labels, resource.fields[0].enum_labels);
});
test("shared native invalid enum label vectors are rejected", () => {
  const invalid = fixture(
    "crates/rom/tests/fixtures/enum-labels-invalid-v1.json",
  );
  assert.ok(Array.isArray(invalid));
  for (const vector of invalid) {
    assert.ok(vector && typeof vector === "object" && !Array.isArray(vector));
    assert.throws(() => parse({ name: "phase", ...vector }));
  }
});
test("enum labels bound UTF8 and preserve prototype-shaped exact members", () => {
  assert.throws(() =>
    parse({
      name: "phase",
      shape: { type: "enum", value: ["x"] },
      enum_labels: { x: "é".repeat(129) },
    }),
  );
  const field = parse({
    name: "phase",
    shape: { type: "enum", value: ["__proto__", "constructor"] },
    enum_labels: Object.fromEntries([
      ["__proto__", "Safe"],
      ["constructor", "Own"],
    ]),
  });
  assert.equal(field.enum_labels?.__proto__, "Safe");
  assert.equal(Object.hasOwn(field.enum_labels!, "__proto__"), true);
});

test("enum label map bounds, omissions, partial labels and invalid container types", () => {
  const shape = { type: "enum", value: ["queued", "running"] };
  assert.equal(parse({ name: "phase", shape }).enum_labels, undefined);
  assert.deepEqual(
    parse({ name: "phase", shape, enum_labels: { queued: "Waiting" } })
      .enum_labels,
    { queued: "Waiting" },
  );
  for (const labels of [
    null,
    [],
    Object.fromEntries(
      Array.from({ length: 1025 }, (_, i) => [`key${i}`, "Label"]),
    ),
  ])
    assert.throws(() => parse({ name: "phase", shape, enum_labels: labels }));
});

test("enum labels do not inherit object prototype or merge duplicate display labels", () => {
  assert.equal(enumLabel("__proto__", {}), "__proto__");
  assert.equal(enumLabel("constructor", {}), "constructor");
  assert.equal(
    enumLabel("running", { running: "Active", paused: "Active" }),
    "Active",
  );
  assert.equal(enumLabel("retired", { running: "Active" }), "retired");
});
