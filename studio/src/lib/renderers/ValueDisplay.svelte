<script lang="ts">
  import type { FieldDescriptor, WireValue } from "../client/types.ts";
  import { enumLabel } from "../client/enum-labels.ts";
  import { findRenderer } from "./registry.ts";
  import { displayValue } from "./value-format.ts";
  import ValueDisplay from "./ValueDisplay.svelte";
  let {
    descriptor,
    value,
    mode = "detail",
    depth = 0,
  }: {
    descriptor: FieldDescriptor;
    value: WireValue | undefined;
    mode?: "cell" | "detail";
    depth?: number;
  } = $props();
  let wrappers = $derived(descriptor.codec_wrappers ?? []);
  let Custom = $derived(
    wrappers.length === 0 ? findRenderer(descriptor.codec) : undefined,
  );
  let inner = $derived(
    "value" in descriptor.shape && !Array.isArray(descriptor.shape.value)
      ? ({
          ...descriptor,
          shape: descriptor.shape.value,
          codec_wrappers: wrappers.slice(1),
        } as FieldDescriptor)
      : descriptor,
  );
  let mismatch = $derived(
    wrappers.length > 0 && wrappers[0] !== descriptor.shape.type,
  );
</script>

{#if mismatch || depth >= 6}<span class="inline-block"
    >{displayValue(value)} (bounded generic display)</span
  >
{:else if (wrappers.length > 0 || (!descriptor.codec && descriptor.enum_labels && descriptor.shape.type !== "enum")) && value !== null && value !== undefined}
  {#if descriptor.shape.type === "optional" || descriptor.shape.type === "nullable"}<ValueDisplay
      descriptor={inner}
      {value}
      {mode}
      depth={depth + 1}
    />
  {:else if descriptor.shape.type === "list" && Array.isArray(value)}<span
      class:inline-block={value.length === 0}
      >[{#each value.slice(0, 100) as item, index}{#if index},
        {/if}<ValueDisplay
          descriptor={inner}
          value={item}
          {mode}
          depth={depth + 1}
        />{/each}{#if value.length > 100}…{/if}]</span
    >
  {:else if descriptor.shape.type === "map" && typeof value === "object" && !Array.isArray(value)}<span
      class:inline-block={Object.keys(value).length === 0}
      >{"{"}{#each Object.entries(value).slice(0, 100) as [key, item], index}{#if index},
        {/if}{key}: <ValueDisplay
          descriptor={inner}
          value={item}
          {mode}
          depth={depth + 1}
        />{/each}{#if Object.keys(value).length > 100}…{/if}{"}"}</span
    >
  {:else}<span class="inline-block">{displayValue(value)}</span>{/if}
{:else if Custom && value !== undefined}<Custom
    {descriptor}
    {value}
    {mode}
    readonly
    onchange={() => {}}
  />
{:else}<span class="inline-block"
    >{descriptor.shape.type === "enum" &&
    typeof value === "string" &&
    descriptor.shape.value.includes(value)
      ? enumLabel(value, descriptor.enum_labels)
      : displayValue(value)}</span
  >{#if descriptor.codec && value !== null && value !== undefined}<span
      class="codec-note"
    >
      (generic display; {descriptor.codec.name} renderer unavailable)</span
    >{/if}{/if}
