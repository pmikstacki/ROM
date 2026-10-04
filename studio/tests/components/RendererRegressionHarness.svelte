<script lang="ts">
  import ResourceTable from "../../src/lib/resources/ResourceTable.svelte";
  import ResponsiveInspector from "../../src/lib/application/ResponsiveInspector.svelte";
  import SelectAdapter from "../../src/lib/renderers/SelectAdapter.svelte";
  import { Button } from "../../src/lib/components/ui/button/index.js";
  import type {
    FieldDescriptor,
    WireValue,
  } from "../../src/lib/client/types.ts";

  const cases: { name: string; field: FieldDescriptor; value?: WireValue }[] = [
    {
      name: "Scalar text",
      field: { name: "value", shape: { type: "string" } },
      value: "Visible scalar",
    },
    {
      name: "Scalar zero",
      field: { name: "value", shape: { type: "u64" } },
      value: 0n,
    },
    {
      name: "Scalar false",
      field: { name: "value", shape: { type: "bool" } },
      value: false,
    },
    {
      name: "Explicit null",
      field: {
        name: "value",
        shape: { type: "nullable", value: { type: "string" } },
      },
      value: null,
    },
    {
      name: "Missing optional",
      field: {
        name: "value",
        shape: { type: "optional", value: { type: "string" } },
      },
    },
    {
      name: "Empty codec list",
      field: {
        name: "value",
        shape: { type: "list", value: { type: "string" } },
        codec: { name: "unregistered-display", version: 1 },
        codec_wrappers: ["list"],
      },
      value: [],
    },
    {
      name: "Empty codec map",
      field: {
        name: "value",
        shape: { type: "map", value: { type: "string" } },
        codec: { name: "unregistered-display", version: 1 },
        codec_wrappers: ["map"],
      },
      value: {},
    },
  ];
  const mobileFixture = new URL(location.href).searchParams.has("mobile");
  let open = $state(false);
  let trigger = $state<HTMLButtonElement | null>(null);
  let selection = $state("asc");
  let applied = $state("Not applied");
</script>

<main class="p-4">
  {#if mobileFixture}
    <h1>Nested mobile control fixture</h1>
    <Button bind:ref={trigger} onclick={() => (open = true)}
      >Open resource tools</Button
    >
    <ResponsiveInspector bind:open onCloseFocus={() => trigger?.focus()}>
      <section aria-label="Sort draft" class="space-y-4">
        <p class="pt-24">Choose a direction, then apply the draft.</p>
        <SelectAdapter
          label="Direction"
          value={selection}
          options={[
            { value: "asc", label: "Ascending" },
            { value: "desc", label: "Descending" },
          ]}
          onchange={(value) => (selection = value)}
        />
        <Button onclick={() => (applied = selection)}>Apply direction</Button>
        <output aria-label="Applied direction">{applied}</output>
        {#each Array.from({ length: 30 }, (_, i) => i + 1) as index}
          <p>Scrollable resource detail {index}</p>
        {/each}
      </section>
    </ResponsiveInspector>
  {:else}
    <h1>Generic value visibility fixture</h1>
    {#each cases as item}
      <section aria-label={item.name}>
        <h2>{item.name}</h2>
        <ResourceTable
          descriptor={{
            kind: item.name,
            version: 1,
            fields: [item.field],
            actions: [],
            action_inputs: [],
          }}
          rows={[
            {
              key: { kind: item.name, id: "sample" },
              revision: 1n,
              value: item.value === undefined ? {} : { value: item.value },
            },
          ]}
          onselect={() => {}}
        />
      </section>
    {/each}
  {/if}
</main>
