<script lang="ts">
  import type { RendererProps } from "../../src/lib/renderers/types.ts";
  let {
    descriptor,
    value,
    onchange,
    readonly = false,
  }: RendererProps = $props();
</script>

<label
  >{descriptor.name} custom value<input
    aria-label={`${descriptor.name} custom value`}
    value={String(value)}
    disabled={readonly}
    oninput={(event) => onchange(event.currentTarget.value)}
  /></label
>
