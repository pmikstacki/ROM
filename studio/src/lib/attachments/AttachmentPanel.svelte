<script lang="ts">
  import { onDestroy, onMount, untrack } from "svelte";
  import type {
    BlobCapabilities,
    ResourceDescriptor,
  } from "../client/types.ts";
  import { createAttachments, type AttachmentClient } from "./controller.ts";
  import ResourceTable from "../resources/ResourceTable.svelte";
  import ValueDisplay from "../renderers/ValueDisplay.svelte";
  import SelectAdapter from "../renderers/SelectAdapter.svelte";
  import CheckboxAdapter from "../renderers/CheckboxAdapter.svelte";
  import { Button } from "../components/ui/button/index.js";
  import { Input } from "../components/ui/input/index.js";
  let {
    client,
    capabilities,
    descriptor,
  }: {
    client: AttachmentClient;
    capabilities: BlobCapabilities;
    descriptor: ResourceDescriptor;
  } = $props();
  const controller = createAttachments(
    untrack(() => client),
    untrack(() => capabilities),
  );
  let snapshot = $state.raw(controller.state),
    id = $state(""),
    store = $state(untrack(() => capabilities.stores[0] ?? "")),
    file = $state.raw<File | null>(null),
    selected = $state(""),
    confirm = $state(false),
    downloadError = $state("");
  onMount(() => void controller.refresh());
  const unsubscribe = controller.subscribe((next) => (snapshot = next));
  const busy = $derived(
    snapshot.busy ||
      ["preparing", "pending", "unknown"].includes(snapshot.phase),
  );
  $effect(() => {
    if (selected && !snapshot.rows.some((row) => row.key.id === selected))
      selected = "";
  });
  onDestroy(() => {
    unsubscribe();
    controller.dispose();
    file = null;
  });
  async function download() {
    downloadError = "";
    const downloadedId = selected;
    try {
      const bytes = await controller.download(downloadedId);
      const data = new Blob([Uint8Array.from(bytes).buffer], {
        type: "application/octet-stream",
      });
      const url = URL.createObjectURL(data),
        link = document.createElement("a");
      link.href = url;
      link.download =
        downloadedId.replace(/[^a-zA-Z0-9_-]/g, "_").slice(0, 64) + ".bin";
      link.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (error) {
      downloadError =
        error instanceof Error ? error.message : "Download failed.";
    }
  }
</script>

<section class="space-y-4" aria-label="Attachment operations">
  <h1 class="text-xl font-semibold tracking-tight">Attachments</h1>
  <p>
    Maximum {capabilities.limits.blob_bytes} bytes per file. A successful upload commits
    an authorized Resource.
  </p>
  <form
    class="grid gap-4 rounded-lg border bg-card p-4 sm:grid-cols-2"
    onsubmit={(event) => {
      event.preventDefault();
      if (file) void controller.upload(id, store, file);
    }}
  >
    <label
      >Attachment Resource ID<Input bind:value={id} disabled={busy} /></label
    >
    <div class="space-y-1.5">
      <span class="text-sm">Attachment store</span><SelectAdapter
        label="Attachment store"
        value={store}
        disabled={busy}
        onchange={(next) => (store = next)}
        options={capabilities.stores.map((name) => ({
          value: name,
          label: name,
        }))}
      />
    </div>
    <label
      >Attachment file<Input
        type="file"
        disabled={busy}
        onchange={(event) => (file = event.currentTarget.files?.[0] ?? null)}
      /></label
    >
    <Button type="submit" disabled={busy || !file || !id || !store}
      >Upload attachment</Button
    >
  </form>
  <p role="status">Attachment operation: {snapshot.phase}</p>
  {#if snapshot.phase === "unknown"}<p>
      The result is unknown. Retry keeps the same Resource ID, reservation key,
      and file contents.
    </p>
    <Button onclick={() => void controller.retry()}
      >Retry same attachment</Button
    >{/if}
  {#if snapshot.error}<p role="alert">{snapshot.error}</p>{/if}
  <Button disabled={busy} onclick={() => void controller.refresh()}
    >Refresh attachments</Button
  >
  <ResourceTable
    {descriptor}
    rows={snapshot.rows}
    onselect={(row) => {
      selected = row.key.id;
      confirm = false;
    }}
  />
  {#if selected}<section
      class="space-y-3 rounded-lg border p-4"
      aria-label="Selected attachment"
    >
      <h3>Attachment {selected}</h3>
      <Button disabled={busy} onclick={() => void download()}
        >Download attachment</Button
      >
      <CheckboxAdapter
        label={`Confirm detachment of ${selected}`}
        checked={confirm}
        onchange={(next) => (confirm = next)}
        disabled={busy}
      />
      <Button
        variant="destructive"
        disabled={busy || !confirm}
        onclick={() => void controller.detach(selected)}
        >Detach attachment</Button
      >
      {#if downloadError}<p role="alert">{downloadError}</p>{/if}
    </section>{/if}
  {#if snapshot.result?.value}<section
      class="space-y-3 rounded-lg border p-4"
      aria-label="Attachment result"
    >
      <h3>Committed Resource {snapshot.result.key.id}</h3>
      <dl>
        {#each descriptor.fields as field}<dt>{field.name}</dt>
          <dd>
            <ValueDisplay
              descriptor={field}
              value={snapshot.result.value[field.name]}
            />
          </dd>{/each}
      </dl>
    </section>{/if}
</section>
