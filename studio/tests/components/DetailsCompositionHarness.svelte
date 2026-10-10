<script lang="ts">
  import { ResponsiveDetails } from "rom-studio/ui/components";
  let open = $state(false);
  let opener = $state<HTMLButtonElement | null>(null);
  let fallback = $state<HTMLButtonElement | null>(null);
  let showOpener = $state(true);
  let disabledOpener = $state(false);
  let closeLabel = $state("Close inspection details");
  let changes = $state(0);
  let nestedOpen = $state(false);
</script>

<main class="mx-auto max-w-5xl p-4">
  <h1>Inspection details composition</h1>
  {#if showOpener}
    <button bind:this={opener} disabled={disabledOpener} onclick={() => (open = true)}>Open inspection details</button>
  {/if}
  <button bind:this={fallback}>Inspection summary</button>
  <output aria-label="Open changes">{changes}</output>
  <ResponsiveDetails
    bind:open
    onOpenChange={() => changes++}
    title="Inspection details"
    description="Review the selected equipment and retain your draft."
    {closeLabel}
    {opener}
    fallbackFocus={() => fallback?.focus()}
    id="inspection-details"
  >
    <label>Inspection draft<textarea rows="3"></textarea></label>
    <button onclick={() => (showOpener = false)}>Remove opening control</button>
    <button onclick={() => (disabledOpener = true)}>Disable opening control</button>
    <button onclick={() => (closeLabel = "Zamknij szczegóły inspekcji")}>Use Polish close label</button>
    <button onclick={() => (nestedOpen = true)}>Open nested details</button>
    <div style="min-height:1200px">Long inspection context</div>
    <button>Final inspection action</button>
  </ResponsiveDetails>
  <ResponsiveDetails
    bind:open={nestedOpen}
    title="Nested inspection details"
    description="An independent details lifetime."
    closeLabel="Close nested details"
    fallbackFocus={() => fallback?.focus()}
  >
    <button onclick={() => (open = false)}>Close outer details</button>
  </ResponsiveDetails>
</main>
