<script lang="ts">
  import ResourceForm from "../../src/lib/resources/ResourceForm.svelte";
  import ResourceTable from "../../src/lib/resources/ResourceTable.svelte";
  import ActionForm from "../../src/lib/resources/ActionForm.svelte";
  import QueryEditor from "../../src/lib/resources/QueryEditor.svelte";
  import { registerRenderer } from "../../src/lib/renderers/registry.ts";
  import { stringifyWire } from "../../src/lib/client/codec.ts";
  import type { ResourceDescriptor } from "../../src/lib/client/types.ts";
  import CustomCode from "./CustomCode.svelte";
  registerRenderer({ name: "fixture-code", version: 1 }, CustomCode);
  const descriptor: ResourceDescriptor = {
    kind: "fixture",
    version: 1,
    fields: [
      { name: "phase", shape: { type: "enum", value: ["open", "closed"] } },
      {
        name: "linked",
        shape: { type: "reference", value: { kind: "fixture" } },
      },
      { name: "scores", shape: { type: "list", value: { type: "i64" } } },
      { name: "count", shape: { type: "u64" } },
      { name: "done", shape: { type: "bool" } },
      {
        name: "note",
        shape: {
          type: "optional",
          value: { type: "nullable", value: { type: "string" } },
        },
      },
      { name: "tags", shape: { type: "list", value: { type: "string" } } },
      { name: "labels", shape: { type: "map", value: { type: "string" } } },
      {
        name: "code",
        shape: { type: "string" },
        codec: { name: "fixture-code", version: 1 },
      },
    ],
    actions: ["adjust", "opaque"],
    action_inputs: [
      {
        name: "adjust",
        version: 1,
        input: {
          type: "object",
          value: [{ name: "amount", shape: { type: "i64" } }],
        },
      },
      { name: "opaque", version: 1, input: null },
    ],
  };
  let submitted = $state("");
  let actionSubmitted = $state("");
  let opaqueSubmitted = $state("");
  let selected = $state("");
  let query = $state("");
</script>

<main>
  <h1>ROM reusable control tests</h1>
  <p>Test fixtures, not simulated application data.</p>
  <ResourceForm
    {descriptor}
    submit={async (value) => {
      submitted = stringifyWire(value);
    }}
  />
  <output data-testid="submitted">{submitted}</output>
  <ResourceTable
    {descriptor}
    rows={[
      {
        key: { kind: "fixture", id: "row-1" },
        revision: 9007199254740993n,
        value: { count: 18446744073709551615n, done: false },
      },
    ]}
    onselect={(row) => (selected = row.key.id)}
  />
  <output data-testid="selected">{selected}</output>
  <ActionForm
    {descriptor}
    action={descriptor.action_inputs[0]}
    oninvoke={async (value) => {
      actionSubmitted = stringifyWire(value);
    }}
  />
  <ActionForm
    {descriptor}
    action={descriptor.action_inputs[1]}
    oninvoke={async (input) => {
      opaqueSubmitted = stringifyWire(input);
    }}
  />
  <output data-testid="action-submitted">{actionSubmitted}</output>
  <output data-testid="opaque-submitted">{opaqueSubmitted}</output>
  <QueryEditor
    {descriptor}
    onchange={(value) => (query = stringifyWire(value as never))}
  /><output data-testid="query-submitted">{query}</output>
</main>
