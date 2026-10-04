<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type {
    RomClient,
    BlobCapabilities,
    ResourceDescriptor,
  } from "../client/types.ts";
  import AttachmentPanel from "../attachments/AttachmentPanel.svelte";
  import { Button } from "../components/ui/button/index.js";
  let {
    client,
    descriptors,
  }: { client: RomClient; descriptors: ResourceDescriptor[] } = $props();
  let capability = $state.raw<BlobCapabilities | null>(null),
    error = $state(""),
    busy = $state(false),
    disposed = false;
  const requests = new AbortController();
  let descriptor = $derived(
    descriptors.find((value) => value.kind === capability?.resource_kind),
  );
  async function connect() {
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

{#if busy}<p role="status">
    Loading attachment capabilities…
  </p>{:else if capability && descriptor}<AttachmentPanel
    {client}
    capabilities={capability}
    {descriptor}
  />{:else}<h1 class="text-xl font-semibold tracking-tight">Attachments</h1>
  <p>
    No authorized attachment capability and Resource descriptor are available.
  </p>
  <Button onclick={() => void connect()}>Check attachment capabilities</Button
  >{/if}
{#if error}<p role="alert">{error}</p>{/if}
