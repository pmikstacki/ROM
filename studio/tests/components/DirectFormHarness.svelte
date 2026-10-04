<script lang="ts">
  import ResourceForm from "../../src/lib/resources/ResourceForm.svelte";
  import { stringifyWire } from "../../src/lib/client/codec.ts";
  import type {
    ResourceDescriptor,
    WireObject,
  } from "../../src/lib/client/types.ts";
  let descriptor = $state.raw<ResourceDescriptor>({
    kind: "field-fixture",
    version: 1,
    fields: [
      { name: "title", shape: { type: "string" } },
      { name: "enabled", shape: { type: "bool" } },
      { name: "count", shape: { type: "u64" } },
      { name: "note", shape: { type: "nullable", value: { type: "string" } } },
      {
        name: "marker",
        shape: { type: "optional", value: { type: "string" } },
      },
      {
        name: "nullable_null",
        shape: { type: "nullable", value: { type: "string" } },
      },
      {
        name: "optional_missing",
        shape: { type: "optional", value: { type: "string" } },
      },
      {
        name: "nullable_values",
        shape: {
          type: "list",
          value: { type: "nullable", value: { type: "string" } },
        },
      },
      {
        name: "large_values",
        shape: { type: "list", value: { type: "string" } },
      },
      {
        name: "opaque",
        shape: { type: "string" },
        codec: { name: "unknown-opaque", version: 1 },
      },
    ],
    actions: [],
    action_inputs: [],
  });
  const current: WireObject = {
    title: "original",
    enabled: false,
    count: 0n,
    note: "present",
    marker: "present",
    nullable_null: null,
    nullable_values: [null, ""],
    large_values: Array.from({ length: 101 }, (_, index) => `item-${index}`),
    opaque: "secret",
  };
  let submitted = $state("");
</script>

<main class="max-w-md p-4">
  <button
    onclick={() =>
      (descriptor = { ...descriptor, version: descriptor.version + 1 })}
    >Change descriptor version</button
  >
  <ResourceForm
    {descriptor}
    value={current}
    direct
    submit={async (input) => (submitted = stringifyWire(input))}
  />
  <output data-testid="direct-submitted">{submitted}</output>
</main>
