<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { Button } from "rom-studio/controls";
  import { SelectionCard } from "rom-studio/ui/components";
  import { createPublicGuideHost } from "./public-guides.ts";
  import type { PublicGuideState } from "./public-guides.ts";
  import { field } from "./model.ts";
  let { generation }: { generation: number } = $props();
  let guides = $state.raw<PublicGuideState>(null!);
  const host = createPublicGuideHost(
    untrack(() => generation),
    (next) => (guides = next),
  );
  guides = host.state;
  const urls = new Set<string>();
  onMount(() => {
    void host.start();
    const visibility = () =>
      host.visible(document.visibilityState !== "hidden");
    document.addEventListener("visibilitychange", visibility);
    return () => {
      document.removeEventListener("visibilitychange", visibility);
      host.dispose();
      urls.forEach((url) => URL.revokeObjectURL(url));
      urls.clear();
    };
  });
  function download() {
    const captured = host.download();
    if (!captured || !host.isCurrent(captured.context)) return;
    const url = URL.createObjectURL(
      new Blob([captured.text], { type: "application/json" }),
    );
    urls.add(url);
    if (!host.isCurrent(captured.context)) {
      URL.revokeObjectURL(url);
      urls.delete(url);
      return;
    }
    const link = document.createElement("a");
    link.href = url;
    link.download = "selected-public-guides.json";
    link.click();
    setTimeout(() => {
      URL.revokeObjectURL(url);
      urls.delete(url);
    }, 1000);
  }
</script>

<section aria-label="Public maintenance guides" class="grid min-w-0 gap-3">
  <h2 class="font-semibold">Public maintenance guides</h2>
  <p>
    Select published guides and add their nominal values. The result is the
    arithmetic total of the selected guide snapshots.
  </p>
  <p>
    Public access needs no sign-in. Computed results are kept only for this
    visit.
  </p>
  <output aria-label="Public guide observation">{guides.phase}</output>
  {#if guides.error}<p role="status">{guides.error}</p>{/if}
  {#each guides.rows as row (row.key.id)}
    <SelectionCard
      id={row.key.id}
      title={field(row, "title")}
      selected={guides.selected.includes(row.key.id)}
      authorityToken={generation}
      failedLabel="Public selection unavailable"
      unknownLabel="Public selection outcome unknown"
      onToggle={(id) => host.toggle(id)}
    >
      <span>Nominal value: {String(row.value?.nominal_voltage)}</span>
      <span>
        · Updated: {field(row, "updated_at")} · Revision: {String(
          row.revision,
        )}</span
      >
    </SelectionCard>
  {/each}
  <Button disabled={!guides.selected.length} onclick={() => host.compute()}
    >Compute selected nominal values</Button
  >
  <output aria-label="Selected nominal value sum">{guides.sum || "none"}</output
  >
  <Button
    disabled={!guides.selected.length || guides.exportPhase === "pending"}
    onclick={() => void host.exportSelected()}
    >Export selected public guide JSON</Button
  >
  {#if guides.exportPhase === "ready"}<Button
      variant="outline"
      onclick={download}>Download public guide JSON</Button
    >{/if}
  <output class="break-all text-xs" aria-label="Public export result"
    >{guides.exportText || "none"}</output
  >
</section>
