<!-- Adapted from SVAR Filter editor/Rule.svelte at 1c581c3312c626c525ee64b8f94446a025fa141c. MIT, see LICENSE. -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { Button } from "../../components/ui/button/index.js";
  let {
    ordinal,
    fieldLabel,
    operatorLabel,
    disabled,
    onremove,
    children,
  }: {
    ordinal: number;
    fieldLabel: string;
    operatorLabel: string;
    disabled: boolean;
    onremove: () => void;
    children: Snippet;
  } = $props();
</script>

<div class="min-w-0 space-y-3 rounded-lg border bg-card p-3">
  <div class="flex min-w-0 items-start justify-between gap-2">
    <span class="min-w-0 text-xs text-muted-foreground">
      <span class="font-medium text-foreground">{fieldLabel}</span>
      <span>{operatorLabel}</span>
    </span>
    <Button
      type="button"
      variant="ghost"
      size="sm"
      {disabled}
      aria-label={`Remove filter ${ordinal}`}
      onclick={onremove}>Remove</Button
    >
  </div>
  {@render children()}
</div>
