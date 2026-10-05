import { readFileSync } from "node:fs";
import { parseWire } from "../../src/lib/client/serialization.ts";
import { test } from "node:test";
import assert from "node:assert/strict";
import { discovery } from "../../src/lib/client/discovery.ts";
import type { WireValue } from "../../src/lib/client/types.ts";
const presentation = {
  label: "Human",
  title_field: "name",
  fields: {
    name: {
      label: "Full name",
      help: "Public display name",
      group: "identity",
    },
  },
  groups: [{ name: "identity", label: "Identity" }],
};
function parse(
  p: WireValue,
  field: WireValue = { name: "name", shape: { type: "string" } },
) {
  return discovery({
    version: 1,
    resources: [
      {
        kind: "human",
        version: 1,
        fields: [field],
        actions: [],
        action_inputs: [],
        presentation: p,
      },
    ],
  }).resources[0];
}
test("discovery retains typed Resource and field presentation", () => {
  assert.deepEqual(
    (parse(presentation) as unknown as { presentation: unknown }).presentation,
    {
      ...presentation,
      fields: Object.assign(Object.create(null), presentation.fields),
    },
  );
});
for (const [name, p] of [
  ["UTF-8 label", { ...presentation, label: "é".repeat(129) }],
  ["empty label", { ...presentation, label: "" }],
  ["unknown selector", { ...presentation, title_field: "secret" }],
  [
    "hidden field",
    { ...presentation, fields: { secret: { label: "Secret" } } },
  ],
  [
    "unknown group",
    { ...presentation, fields: { name: { group: "unknown" } } },
  ],
  [
    "duplicate group",
    {
      ...presentation,
      groups: [...presentation.groups, ...presentation.groups],
    },
  ],
  [
    "unknown executable hint",
    { ...presentation, render: "https://untrusted.example/code.js" },
  ],
] as const)
  test(`discovery rejects invalid presentation ${name}`, () => {
    assert.throws(() => parse(p as unknown as WireValue), /presentation/);
  });
test("title selectors cannot reinterpret custom codecs or nontext values", () => {
  assert.throws(
    () => parse(presentation, { name: "name", shape: { type: "bool" } }),
    /presentation/,
  );
  assert.throws(
    () =>
      parse(presentation, {
        name: "name",
        shape: { type: "string" },
        codec: { name: "secret", version: 1 },
      }),
    /presentation/,
  );
});
test("legacy descriptors omit presentation without changing discovery version", () => {
  const legacy = {
    kind: "legacy",
    version: 1,
    fields: [{ name: "name", shape: { type: "string" } }],
    actions: [],
    action_inputs: [],
  };
  const parsed = discovery({ version: 1, resources: [legacy] });
  assert.equal(parsed.version, 1);
  assert.equal(parsed.resources[0].kind, legacy.kind);
  assert.deepEqual(parsed.resources[0].fields, legacy.fields);
  assert.equal(parsed.resources[0].presentation, undefined);
});

test("client accepts the exact backend discovery conformance fixture", () => {
  const value = parseWire(
    readFileSync(
      new URL("../fixtures/presentation-discovery.json", import.meta.url),
      "utf8",
    ),
  );
  const resource = discovery(value).resources[0];
  assert.equal(resource.presentation?.label, "Human");
  assert.equal(resource.presentation?.title_field, "name");
  assert.equal(resource.presentation?.settings?.group, "people");
  assert.equal(resource.presentation?.fields?.name.label, "Full name");
});
