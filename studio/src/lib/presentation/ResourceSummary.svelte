<script lang="ts">
  import type { ResourceDescriptor, ProjectedView } from "../client/types.ts";
  import { resourceTitle, resourceLabel } from "./resource-presentation.ts";
  import { Button } from "../components/ui/button/index.js";
  import CopyIcon from "@lucide/svelte/icons/copy";
  let {
    descriptor,
    resource,
  }: { descriptor: ResourceDescriptor; resource: ProjectedView } = $props();
  let status = $state("");
  const title = $derived(resourceTitle(descriptor, resource));
  async function copyIdentity() {
    try {
      await navigator.clipboard.writeText(resource.key.id);
      status = "Resource ID copied.";
    } catch {
      status = "Copy unavailable. Select the Resource ID below.";
    }
  }
</script>

<div class="min-w-0 space-y-1.5">
  <h2 class="truncate text-lg font-semibold tracking-tight" {title}>{title}</h2>
  <p class="text-xs text-muted-foreground">
    {resourceLabel(descriptor)} · Revision {String(resource.revision)}
  </p>
  <div class="flex min-w-0 items-center gap-1.5">
    <code
      class="min-w-0 flex-1 select-all break-all text-xs text-muted-foreground"
      aria-label="Resource ID">{resource.key.id}</code
    >
    <Button
      type="button"
      variant="ghost"
      size="icon-sm"
      aria-label="Copy Resource ID"
      onclick={() => void copyIdentity()}><CopyIcon class="size-3.5" /></Button
    >
  </div>
  {#if status}<p role="status" class="text-xs text-muted-foreground">
      {status}
    </p>{/if}
</div>
