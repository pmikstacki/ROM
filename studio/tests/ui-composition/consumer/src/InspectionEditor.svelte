<script lang="ts">
  import { Button, Input, Label, Textarea } from "rom-studio/controls";
  import { ReferencePicker } from "rom-studio/ui/components";
  import type { WireValue } from "rom-studio/client";
  import type { PortalHost, PortalState } from "./host.ts";
  let {
    host,
    state,
    notes = $bindable(""),
    date = $bindable(""),
    equipment = $bindable<WireValue>(""),
  }: {
    host: PortalHost;
    state: PortalState;
    notes?: string;
    date?: string;
    equipment?: WireValue;
  } = $props();
  const recovery = $derived(state.recovery["inspections/check-1"]);
  const blocked = $derived(
    state.subject !== "alice" ||
      state.stored["inspections/check-1"] ||
      recovery?.hasUnresolvedIntent ||
      recovery?.phase === "submitting",
  );
</script>

<div class="grid min-w-0 gap-3">
  <Label for="inspection-date">Inspection UTC instant</Label>
  <Input
    id="inspection-date"
    type="text"
    bind:value={date}
    placeholder="2026-10-09T14:30:00Z"
    disabled={state.subject !== "alice"}
  />
  <p class="text-xs text-muted-foreground">
    Enter an ISO 8601 instant with an explicit UTC offset. The server validates
    the date.
  </p>
  <ReferencePicker
    kind="equipment"
    value={equipment}
    label="Related equipment"
    lookup={state.lookup ?? undefined}
    authorityToken={state.generation}
    onchange={(value) => (equipment = value)}
    readonly={state.subject !== "alice"}
  />
  <Label for="inspection-notes">Inspection notes</Label>
  <Textarea
    id="inspection-notes"
    bind:value={notes}
    rows={3}
    maxlength={4096}
    class="portal-notes"
    disabled={state.subject !== "alice"}
  />
  <Button
    disabled={!!blocked}
    onclick={() => void host.saveInspection(notes, date, equipment)}
    >Save inspection</Button
  >
  <output aria-label="Inspection save">{recovery?.phase ?? "idle"}</output>
  {#if state.stored["inspections/check-1"]}
    <Button
      variant="outline"
      onclick={() => void host.restore("inspections", "check-1")}
      >Restore pending inspection</Button
    >
  {/if}
  {#if recovery?.hasUnresolvedIntent && recovery.phase !== "submitting"}
    <Button
      variant="outline"
      onclick={() => void host.retry("inspections", "check-1")}
      >Retry original inspection</Button
    >
    <p role="status">
      The original save may have committed. Retry keeps its exact invocation.
    </p>
  {/if}
</div>

<style>
  :global(.portal-notes) {
    field-sizing: fixed;
    min-height: 5rem;
    height: 5rem;
    max-height: 5rem;
    overflow-y: auto;
  }
</style>
