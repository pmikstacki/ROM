<script lang="ts">
  import type { FieldDescriptor, WireValue } from "../client/types.ts";
  import { findRenderer } from "./registry.ts";
  import { displayValue } from "./value-format.ts";
  let {
    descriptor,
    value,
    mode = "detail",
  }: {
    descriptor: FieldDescriptor;
    value: WireValue | undefined;
    mode?: "cell" | "detail";
  } = $props();
  let Custom = $derived(findRenderer(descriptor.codec));
</script>

{#if Custom && value !== undefined}<Custom
    {descriptor}
    {value}
    {mode}
    readonly
    onchange={() => {}}
  />{:else}<span>{displayValue(value)}</span>{#if descriptor.codec}<span
      class="codec-note"
    >
      (generic display; {descriptor.codec.name} renderer unavailable)</span
    >{/if}{/if}
