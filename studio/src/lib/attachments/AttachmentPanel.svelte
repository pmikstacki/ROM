<script lang="ts">
  import { onDestroy, onMount, untrack } from "svelte";
  import type {
    BlobCapabilities,
    ResourceDescriptor,
  } from "../client/types.ts";
  import { createAttachments, type AttachmentClient } from "./controller.ts";
  import ResourceTable from "../resources/ResourceTable.svelte";
  import SelectAdapter from "../renderers/SelectAdapter.svelte";
  import ResponsiveInspector from "../application/ResponsiveInspector.svelte";
  import InspectorToggle from "../application/InspectorToggle.svelte";
  import AttachmentDetails from "./AttachmentDetails.svelte";
  import { Button } from "../components/ui/button/index.js";
  import { Input } from "../components/ui/input/index.js";
  import { Upload, RefreshCw, RotateCcw, Paperclip } from "@lucide/svelte";
  let {
    client,
    capabilities,
    descriptor,
    onNavigationBlockChange = () => {},
  }: {
    client: AttachmentClient;
    capabilities: BlobCapabilities;
    descriptor: ResourceDescriptor;
    onNavigationBlockChange?: (blocked: boolean) => void;
  } = $props();
  const controller = createAttachments(
    untrack(() => client),
    untrack(() => capabilities),
  );
  const componentId = $props.id(),
    inspectorId = `attachment-inspector-${componentId}`;
  let snapshot = $state.raw(controller.state),
    id = $state(""),
    store = $state(untrack(() => capabilities.stores[0] ?? "")),
    file = $state.raw<File | null>(null),
    selected = $state(""),
    confirm = $state(false),
    downloadError = $state(""),
    refreshing = $state(false),
    loaded = $state(false),
    inspectorOpen = $state(false),
    lastOperation = $state<"upload" | "detach">("upload");
  let inspectorTrigger = $state<HTMLButtonElement | null>(null),
    inspectorOpener = $state<HTMLButtonElement | null>(null);
  const unsubscribe = controller.subscribe((next) => (snapshot = next));
  const busy = $derived(
    snapshot.busy ||
      ["preparing", "pending", "unknown"].includes(snapshot.phase),
  );
  const selectedRow = $derived(
    snapshot.rows.find((row) => row.key.id === selected) ?? null,
  );
  const canUpload = $derived(
    capabilities.operations.includes("reserve") &&
      capabilities.operations.includes("upload") &&
      capabilities.stores.length > 0,
  );
  const canDownload = $derived(capabilities.operations.includes("download"));
  const canDetach = $derived(capabilities.operations.includes("detach"));
  const status = $derived(
    snapshot.phase === "idle"
      ? "Ready to upload or inspect attachments."
      : snapshot.phase === "preparing"
        ? "Preparing your file…"
        : snapshot.phase === "pending"
          ? lastOperation === "detach"
            ? "Detaching attachment…"
            : "Uploading attachment…"
          : snapshot.phase === "unknown"
            ? "Attachment outcome unknown"
            : snapshot.phase === "success"
              ? lastOperation === "detach"
                ? "Attachment detached"
                : "Attachment committed"
              : "Attachment operation failed",
  );
  $effect(() => onNavigationBlockChange(busy || snapshot.pending !== null));
  $effect(() => {
    if (selected && !selectedRow) {
      selected = "";
      confirm = false;
    }
  });
  onMount(() => void refresh());
  onDestroy(() => {
    unsubscribe();
    controller.dispose();
    file = null;
  });
  async function refresh() {
    refreshing = true;
    await controller.refresh();
    refreshing = false;
    loaded = true;
  }
  async function upload() {
    if (!file || !canUpload) return;
    lastOperation = "upload";
    await controller.upload(id, store, file);
  }
  async function detach() {
    if (!selected || !confirm || !canDetach) return;
    const exactId = selected;
    confirm = false;
    lastOperation = "detach";
    await controller.detach(exactId);
  }
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
      if (selected === downloadedId)
        downloadError =
          error instanceof Error ? error.message : "Download failed.";
    }
  }
</script>

<section class="min-w-0 space-y-4" aria-label="Attachment operations">
  <div class="flex flex-wrap items-center gap-2">
    <Paperclip
      class="size-5 shrink-0 text-muted-foreground"
      aria-hidden="true"
    />
    <h1 class="min-w-0 flex-1 break-words text-xl font-semibold tracking-tight">
      Attachments
    </h1>
    <div class="ml-auto flex items-center gap-2">
      <Button
        variant="outline"
        size="sm"
        class="max-sm:size-9"
        aria-label="Refresh attachments"
        disabled={busy || refreshing}
        onclick={() => void refresh()}
        ><RefreshCw aria-hidden="true" /><span class="hidden sm:inline"
          >Refresh</span
        ></Button
      >
      <InspectorToggle
        bind:ref={inspectorTrigger}
        open={inspectorOpen}
        controls={inspectorId}
        label="attachment details panel"
        onclick={() => {
          inspectorOpener = inspectorTrigger;
          inspectorOpen = !inspectorOpen;
        }}
      />
    </div>
  </div>
  <p class="text-sm text-muted-foreground">
    Upload a file, then inspect its authorized attachment details. Maximum {capabilities.limits.blob_bytes.toLocaleString()}
    bytes per file.
  </p>
  <form
    class="grid min-w-0 grid-cols-[minmax(0,1fr)] gap-3 rounded-lg border bg-card p-4 sm:grid-cols-2"
    onsubmit={(event) => {
      event.preventDefault();
      void upload();
    }}
  >
    <label class="min-w-0 space-y-1.5 text-sm"
      >Attachment Resource ID<Input
        bind:value={id}
        disabled={busy || !canUpload}
      /></label
    >
    <div class="min-w-0 space-y-1.5 [&_[data-slot=select-trigger]]:min-w-0">
      <span class="text-sm">Attachment store</span><SelectAdapter
        label="Attachment store"
        value={store}
        disabled={busy || !canUpload}
        onchange={(next) => (store = next)}
        options={capabilities.stores.map((name) => ({
          value: name,
          label: name,
        }))}
      />
    </div>
    <label class="min-w-0 space-y-1.5 text-sm"
      >Attachment file<Input
        type="file"
        disabled={busy || !canUpload}
        onchange={(event) => (file = event.currentTarget.files?.[0] ?? null)}
      /></label
    >
    <div class="flex items-end">
      <Button
        type="submit"
        class="h-auto min-h-9 max-sm:w-full whitespace-normal"
        aria-label="Upload attachment"
        disabled={busy || !canUpload || !file || !id || !store}
        ><Upload aria-hidden="true" /><span>Upload attachment</span></Button
      >
    </div>
    {#if !canUpload}<p class="text-sm text-muted-foreground sm:col-span-2">
        Upload is unavailable for this session.
      </p>{/if}
  </form>
  <p role="status" class="text-sm">{status}</p>
  {#if snapshot.phase === "unknown"}<section
      class="space-y-3 rounded-lg border border-amber-500/40 p-3"
      aria-label="Unknown attachment outcome"
    >
      <p class="text-sm">
        {snapshot.pending?.step === "detach"
          ? "Detachment was not confirmed. Retry keeps the same attachment ID."
          : "Upload was not confirmed. Retry keeps the same Resource ID, reservation key, and file contents."}
        Resolve this outcome before leaving Attachments.
      </p>
      <Button
        variant="outline"
        class="h-auto min-h-9 whitespace-normal"
        onclick={() => void controller.retry()}
        disabled={snapshot.busy}
        aria-label="Retry same attachment"
        ><RotateCcw aria-hidden="true" /><span>Retry same attachment</span
        ></Button
      >
    </section>{/if}
  {#if snapshot.error}<p
      role="alert"
      class="break-words text-sm text-destructive"
    >
      {snapshot.error}
    </p>{/if}
  <div
    class={inspectorOpen
      ? "grid min-w-0 items-start gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(20rem,26rem)]"
      : "min-w-0"}
  >
    <section class="min-w-0" aria-label="Attachment list">
      {#if refreshing}<p
          role="status"
          class="py-6 text-sm text-muted-foreground"
        >
          Loading attachments…
        </p>
      {:else if loaded && snapshot.rows.length === 0 && !snapshot.error}<p
          class="py-6 text-sm text-muted-foreground"
        >
          No authorized attachments are available.
        </p>
      {:else if snapshot.rows.length > 0}<ResourceTable
          {descriptor}
          rows={snapshot.rows}
          onselect={(row, opener) => {
            selected = row.key.id;
            confirm = false;
            downloadError = "";
            inspectorOpener = opener ?? inspectorTrigger;
            inspectorOpen = true;
          }}
        />{/if}
    </section>
    <ResponsiveInspector
      bind:open={inspectorOpen}
      id={inspectorId}
      title="Attachment details"
      label="Attachment details"
      description="Authorized details and operations for the selected attachment."
      onCloseFocus={() => {
        if (inspectorOpener?.isConnected) inspectorOpener.focus();
        else inspectorTrigger?.focus();
      }}
    >
      <AttachmentDetails
        {descriptor}
        row={selectedRow}
        bind:confirm
        {busy}
        {canDownload}
        {canDetach}
        onDownload={() => void download()}
        onDetach={() => void detach()}
        {downloadError}
      />
      {#if snapshot.result}<p
          class="mt-4 break-all border-t pt-3 text-xs text-muted-foreground"
        >
          Committed attachment ID: {snapshot.result.key.id}
        </p>{/if}
    </ResponsiveInspector>
  </div>
</section>
