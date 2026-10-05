<script lang="ts">
  import type { RendererProps } from "../../studio/src/lib/renderers/types.ts";
  import { Input } from "../../studio/src/lib/components/ui/input/index.js";
  let {
    value,
    onchange,
    readonly = false,
    descriptor,
    mode = "editor",
    onerror,
    label,
  }: RendererProps = $props();
  function change(next: string) {
    const canonical = next.trim().toUpperCase();
    onchange(canonical);
    onerror?.(
      /^TICKET-[A-Z0-9-]{1,25}$/.test(canonical) && canonical.length >= 8
        ? ""
        : "Use a TICKET- code with uppercase letters, digits, or hyphens.",
    );
  }
</script>

{#if mode === "editor"}<Input
    value={typeof value === "string" ? value : ""}
    aria-label={`${label || descriptor.name} value`}
    {readonly}
    oninput={(event) => change(event.currentTarget.value)}
  />{:else}<span>{String(value)}</span>{/if}
