<script lang="ts">
  import ResourceForm from "../../src/lib/resources/ResourceForm.svelte";
  import ResourceTable from "../../src/lib/resources/ResourceTable.svelte";
  import ActionForm from "../../src/lib/resources/ActionForm.svelte";
  import QueryEditor from "../../src/lib/resources/QueryEditor.svelte";
  import { registerRenderer } from "../../src/lib/renderers/registry.ts";
  import { stringifyWire } from "../../src/lib/client/codec.ts";
  import type {
    ResourceDescriptor,
    ProjectedView,
  } from "../../src/lib/client/types.ts";
  import CustomCode from "./CustomCode.svelte";
  registerRenderer({ name: "fixture-code", version: 1 }, CustomCode);
  let descriptor = $state.raw<ResourceDescriptor>({
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
      {
        name: "maybe_code",
        shape: {
          type: "optional",
          value: { type: "nullable", value: { type: "string" } },
        },
        codec: { name: "fixture-code", version: 1 },
        codec_wrappers: ["optional", "nullable"],
      },
      {
        name: "codes",
        shape: { type: "list", value: { type: "string" } },
        codec: { name: "fixture-code", version: 1 },
        codec_wrappers: ["list"],
      },
      {
        name: "code_map",
        shape: { type: "map", value: { type: "string" } },
        codec: { name: "fixture-code", version: 1 },
        codec_wrappers: ["map"],
      },
      {
        name: "unknown_code",
        shape: { type: "optional", value: { type: "string" } },
        codec: { name: "absent-code", version: 1 },
        codec_wrappers: ["optional"],
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
  });
  const requestedRows = new URL(location.href).searchParams.get(
    "benchmark_rows",
  );
  const rowCount =
    requestedRows === "100" ? 100 : requestedRows === "500" ? 500 : 1;
  const benchmark = requestedRows === "100" || requestedRows === "500";
  let rows = $state.raw<ProjectedView[]>(
    Array.from({ length: rowCount }, (_, index) => ({
      key: { kind: "fixture", id: "row-" + (index + 1) },
      revision: 9007199254740993n,
      value: {
        count: 18446744073709551615n,
        done: false,
        maybe_code: "DETAIL",
        codes: ["LEAF"],
        code_map: { primary: "ENTRY" },
      },
    })),
  );
  let submitted = $state("");
  let actionSubmitted = $state("");
  let opaqueSubmitted = $state("");
  let selected = $state("");
  let query = $state("");
  let formGeneration = $state(0);
</script>

<main>
  <h1>ROM reusable control tests</h1>
  <p>Test fixtures, not simulated application data.</p>
  <button
    onclick={() =>
      (descriptor = { ...descriptor, version: descriptor.version + 1 })}
    >Advance fixture descriptor</button
  >
  <button onclick={() => formGeneration++}
    >Discard draft and reopen fixture</button
  >
  {#key formGeneration}<ResourceForm
      {descriptor}
      submit={async (value) => {
        submitted = stringifyWire(value);
      }}
    />{/key}
  <output data-testid="submitted">{submitted}</output>
  <ResourceTable
    {descriptor}
    {rows}
    onselect={(row) => (selected = row.key.id)}
  />
  <output data-testid="selected">{selected}</output>
  {#if benchmark}<button onclick={() => (rows = [])}
      >Clear benchmark rows</button
    >{/if}
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
