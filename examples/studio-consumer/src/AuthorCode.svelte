<script lang="ts">
  import { Input, type RendererProps } from "rom-studio";
  let { value, onchange, readonly = false, descriptor, mode = "editor", onerror }: RendererProps = $props();
  function change(next: string) {
    const canonical = next.trim().toUpperCase();
    onchange(canonical);
    onerror?.(/^TICKET-[A-Z0-9-]{1,25}$/.test(canonical) ? "" : "Use a TICKET- code.");
  }
</script>

{#if mode === "editor"}
  <Input value={typeof value === "string" ? value : ""} aria-label={`Author ${descriptor.name} editor`} {readonly} oninput={(event) => change(event.currentTarget.value)} />
{:else}
  <span>Author code: {String(value)}</span>
{/if}
