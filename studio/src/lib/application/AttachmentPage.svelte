<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type {
    RomClient,
    BlobCapabilities,
    ResourceDescriptor,
  } from "../client/types.ts";
  import AttachmentPanel from "../attachments/AttachmentPanel.svelte";
  import { Button } from "../components/ui/button/index.js";
  import { RefreshCw, Paperclip } from "@lucide/svelte";
  let {
    client,
    descriptors,
    onNavigationBlockChange = () => {},
  }: {
    client: RomClient;
    descriptors: ResourceDescriptor[];
    onNavigationBlockChange?: (blocked: boolean) => void;
  } = $props();
  let capability = $state.raw<BlobCapabilities | null>(null),
    error = $state(""),
    busy = $state(false),
    disposed = false;
  const requests = new AbortController();
  let descriptor = $derived(
    descriptors.find((value) => value.kind === capability?.resource_kind),
  );
  async function connect() {
    if (busy) return;
    busy = true;
    error = "";
    try {
      const value = await client.blobCapabilities(requests.signal);
      if (!disposed) capability = value;
    } catch (problem) {
      if (!disposed) {
        capability = null;
        error =
          problem instanceof Error
            ? problem.message
            : "Attachment transport unavailable.";
      }
    } finally {
      if (!disposed) busy = false;
    }
  }
  onMount(() => void connect());
  onDestroy(() => {
    disposed = true;
    requests.abort();
    capability = null;
  });
</script>

{#if capability && descriptor}
  <AttachmentPanel
    {client}
    capabilities={capability}
    {descriptor}
    {onNavigationBlockChange}
  />
{:else}
  <section class="space-y-4" aria-label="Attachment availability">
    <h1 class="flex items-center gap-2 text-xl font-semibold tracking-tight">
      <Paperclip class="size-5" aria-hidden="true" />{busy
        ? "Attachments"
        : "Attachments unavailable"}
    </h1>
    {#if busy}<p role="status" class="text-sm text-muted-foreground">
        Checking attachment access…
      </p>
    {:else}<p class="text-sm text-muted-foreground">
        This session has no available attachment capability and authorized
        Resource descriptor. Check access to try again.
      </p>{/if}
    {#if error}<p role="alert" class="break-words text-sm text-destructive">
        {error}
      </p>{/if}
    <Button
      variant="outline"
      disabled={busy}
      onclick={() => void connect()}
      aria-label="Check attachment capabilities"
      ><RefreshCw aria-hidden="true" /><span
        >{busy ? "Checking access…" : "Check attachment capabilities"}</span
      ></Button
    >
  </section>
{/if}
