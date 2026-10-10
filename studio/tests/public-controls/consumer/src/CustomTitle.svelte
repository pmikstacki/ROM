<script lang="ts">
  import type { RendererProps } from "rom-studio";
  import { Input } from "rom-studio/controls";
  let { value, onchange, readonly = false, mode, label = "Title" }: RendererProps = $props();
</script>
{#if readonly || mode === "cell" || mode === "detail"}
  <span data-testid="installed-title">{String(value)}</span>
{:else}
  <Input aria-label={label} value={String(value)} oninput={(event) => onchange(event.currentTarget.value)} />
{/if}
