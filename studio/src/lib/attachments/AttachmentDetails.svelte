<script lang="ts">
  import type { ProjectedView, ResourceDescriptor } from "../client/types.ts";
  import {
    resourceTitle,
    resourceLabel,
  } from "../presentation/resource-presentation.ts";
  import ValueDisplay from "../renderers/ValueDisplay.svelte";
  import CheckboxAdapter from "../renderers/CheckboxAdapter.svelte";
  import { Button } from "../components/ui/button/index.js";
  import { Copy, Download, Unlink } from "@lucide/svelte";
  let {
    descriptor,
    row,
    confirm = $bindable(false),
    busy,
    canDownload,
    canDetach,
    onDownload,
    onDetach,
    downloadError = "",
  }: {
    descriptor: ResourceDescriptor;
    row: ProjectedView | null;
    confirm?: boolean;
    busy: boolean;
    canDownload: boolean;
    canDetach: boolean;
    onDownload: () => void;
    onDetach: () => void;
    downloadError?: string;
  } = $props();
  let copyStatus = $state("");
  $effect(() => {
    row?.key.id;
    copyStatus = "";
  });
  async function copy(id: string) {
    try {
      await navigator.clipboard.writeText(id);
      if (row?.key.id === id) copyStatus = "Attachment ID copied.";
    } catch {
      if (row?.key.id === id)
        copyStatus = "Could not copy. Select the attachment ID to copy it.";
    }
  }
</script>

{#if row}
  <section class="min-w-0 space-y-4" aria-label="Selected attachment">
    <div class="space-y-1">
      <p class="text-xs text-muted-foreground">
        {resourceLabel(descriptor)} · Revision {row.revision.toString()}
      </p>
      <h2 class="break-words text-lg font-semibold">
        {resourceTitle(descriptor, row)}
      </h2>
    </div>
    <div class="flex min-w-0 items-start gap-2">
      <div class="min-w-0 flex-1">
        <p class="text-xs text-muted-foreground">Exact attachment ID</p>
        <code class="select-all break-all text-xs">{row.key.id}</code>
      </div>
      <Button
        variant="ghost"
        size="icon"
        aria-label="Copy attachment ID"
        onclick={() => void copy(row!.key.id)}
        ><Copy aria-hidden="true" /></Button
      >
    </div>
    {#if copyStatus}<p role="status" class="text-xs text-muted-foreground">
        {copyStatus}
      </p>{/if}
    <dl class="grid min-w-0 gap-3">
      {#each descriptor.fields as field}
        <div class="min-w-0">
          <dt class="text-xs text-muted-foreground">
            {descriptor.presentation?.fields?.[field.name]?.label || field.name}
          </dt>
          <dd class="min-w-0 break-words">
            <ValueDisplay descriptor={field} value={row.value?.[field.name]} />
          </dd>
        </div>
      {/each}
    </dl>
    <Button
      variant="outline"
      disabled={busy || !canDownload}
      onclick={onDownload}
      aria-label="Download attachment"
      ><Download aria-hidden="true" /><span>Download attachment</span></Button
    >
    {#if !canDownload}<p class="text-sm text-muted-foreground">
        Download is unavailable for this session.
      </p>{/if}
    <section
      class="space-y-3 border-t pt-4"
      aria-label="Detach selected attachment"
    >
      <p class="text-sm text-muted-foreground">
        Detach this attachment from its Resource. Confirm the exact ID before
        continuing.
      </p>
      <CheckboxAdapter
        label={`Confirm detachment of ${row.key.id}`}
        checked={confirm}
        onchange={(next) => (confirm = next)}
        disabled={busy || !canDetach}
      />
      <Button
        variant="destructive"
        disabled={busy || !confirm || !canDetach}
        onclick={onDetach}
        aria-label="Detach attachment"
        ><Unlink aria-hidden="true" /><span>Detach attachment</span></Button
      >
      {#if !canDetach}<p class="text-sm text-muted-foreground">
          Detachment is unavailable for this session.
        </p>{/if}
    </section>
    {#if downloadError}<p
        role="alert"
        class="break-words text-sm text-destructive"
      >
        {downloadError}
      </p>{/if}
  </section>
{:else}<p class="text-sm text-muted-foreground">
    Select an attachment to inspect its authorized details.
  </p>{/if}
