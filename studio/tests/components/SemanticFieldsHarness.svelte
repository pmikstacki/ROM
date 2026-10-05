<script lang="ts">
  import SemanticField from "../../src/lib/renderers/SemanticField.svelte";
  import ValueDisplay from "../../src/lib/renderers/ValueDisplay.svelte";
  import ResourceForm from "../../src/lib/resources/ResourceForm.svelte";
  import ResourceTable from "../../src/lib/resources/ResourceTable.svelte";
  import type {
    ResourceDescriptor,
    WireValue,
  } from "../../src/lib/client/types.ts";
  import { stringifyWire } from "../../src/lib/client/codec.ts";
  const kinds = [
    "date",
    "time",
    "datetime",
    "color",
    "email",
    "url",
    "multiline",
    "json-document",
    "decimal",
    "unit-value",
  ];
  const descriptor: ResourceDescriptor = {
    kind: "measurement",
    version: 1,
    fields: kinds.map((kind) => ({
      name: kind,
      shape:
        kind === "unit-value"
          ? { type: "map", value: { type: "string" } }
          : { type: "string" },
      codec: { name: `rom.${kind}`, version: 1 },
    })),
    actions: [],
    action_inputs: [],
  };
  const initial: Record<string, WireValue> = {
    date: "2024-02-29",
    time: "12:00:00.123456789",
    datetime: "2026-10-05T10:00:00.000000000Z",
    color: "#112233aa",
    email: "alice@example.com",
    url: "https://example.com/",
    multiline: "one\ntwo",
    "json-document": '{"number":9007199254740993}',
    decimal: "9007199254740993.000000000000000001",
    "unit-value": { value: "12.3", unit: "kg" },
  };
  let submitted = $state("null");
</script>

<div class="space-y-4 p-4">
  <ResourceForm
    {descriptor}
    value={initial}
    mode="patch"
    direct
    submit={async (patch) => {
      submitted = stringifyWire(patch);
    }}
  />
  <output aria-label="Semantic patch">{submitted}</output>
  <ResourceTable
    {descriptor}
    rows={[
      { key: { kind: "measurement", id: "one" }, revision: 1n, value: initial },
    ]}
    onselect={() => {}}
  />
</div>

<section aria-label="Read-only semantic details" class="p-4">
  <ValueDisplay descriptor={{name:"Archived document",shape:{type:"string"},codec:{name:"rom.json-document",version:1}}} value={'{"exact":9007199254740993,"notes":"'+"complete ".repeat(30)+'END"}'} mode="detail" />
</section>

<section aria-label="Compact measurement" style="width: 130px">
<SemanticField descriptor={{name:"Compact measurement",shape:{type:"map",value:{type:"string"}},codec:{name:"rom.unit-value",version:1}}} value={{value:"12.5",unit:"kg"}} onchange={()=>{}} />
</section>
