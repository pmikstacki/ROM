<script lang="ts">
  import type { RendererProps } from "../../src/lib/renderers/types.ts";
  import { Input } from "../../src/lib/components/ui/input/index.js";
  let {
    value,
    label,
    descriptor,
    readonly = false,
    onchange,
  }: RendererProps = $props();
</script>

<Input
  aria-label={`${label ?? descriptor.name} registered value`}
  value={String(value)}
  disabled={readonly}
  oninput={(event) => onchange(event.currentTarget.value)}
/>
