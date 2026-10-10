<script lang="ts">
  import { getContext } from "svelte";
  import ReferencePicker from "../ui/components/ReferencePicker.svelte";
  import {
    REFERENCE_LOOKUP,
    type ReferenceLookup,
  } from "./reference-lookup.ts";
  import type { WireValue } from "../client/types.ts";
  let {
    kind,
    value,
    label,
    path = label,
    onchange,
    onerror = () => {},
    readonly = false,
    showLabel = true,
  }: {
    kind: string;
    value: WireValue;
    label: string;
    path?: string;
    onchange: (value: WireValue) => void;
    onerror?: (message: string) => void;
    readonly?: boolean;
    showLabel?: boolean;
  } = $props();
  const lookup = getContext<ReferenceLookup | undefined>(REFERENCE_LOOKUP);
  const authorityToken = $derived(lookup?.descriptor(kind));
</script>

<ReferencePicker
  {kind}
  {value}
  {label}
  {path}
  {onchange}
  {onerror}
  {readonly}
  {showLabel}
  {lookup}
  {authorityToken}
/>
