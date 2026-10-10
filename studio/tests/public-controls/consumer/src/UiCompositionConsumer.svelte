<script lang="ts">
  import { onMount } from "svelte";
  import { ConversationLayout, ResponsiveDetails, HistoryList, ReferencePicker, SelectionCard, LayoutControls } from "rom-studio/ui/components";
  import type { LayoutItem, LayoutOptions } from "rom-studio/ui";
  import type { WireValue } from "rom-studio/client";
  import { createLatestRequest, createSelection } from "rom-studio/ui";
  import type { LatestRequestState } from "rom-studio/ui";
  import { createObservation } from "rom-studio/observe";
  import type { ObservationState, ObservationSource } from "rom-studio/observe";
  import { Button, Input, Label, Textarea } from "rom-studio/controls";

  let conversationDraft = $state("");
  let conversationExpanded = $state(false);
  let conversationSubmissions = $state(0);
  function submitConversation() {
    if (!conversationDraft.trim()) return;
    conversationSubmissions++;
    conversationDraft = "";
  }
  type Row = { label: string };
  let open = $state(false);
  let draft = $state("inspection draft");
  let opener = $state<HTMLButtonElement | HTMLAnchorElement | null>(null);
  let fallback = $state<HTMLButtonElement | HTMLAnchorElement | null>(null);
  let selected = $state<readonly string[]>([]);
  const selection = createSelection({ ids: ["pump-1", "valve-1"], initial: [], multiple: true });
  let latest = $state<LatestRequestState<string>>({ phase: "idle", identity: null, value: null, code: null });
  const requests = createLatestRequest({ clone: (value: string) => value, classifyError: () => "Unavailable", onState: (value) => { latest = value; } });
  let releaseOld: (() => void) | undefined;
  let observationState = $state<ObservationState<Row> | null>(null);
  let lost = $state(0);
  let generation = $state(0);
  let reference = $state<WireValue>("pump-1");
  let historyId = $state<string | null>(null);
  let layout = $state<readonly LayoutItem[]>([{ id: "history", x: 0, y: 0, width: 2, height: 1, visible: true }]);
  const layoutOptions: LayoutOptions = { columns: 4, maxRows: 4, maxItems: 1, catalog: [{ id: "history", minWidth: 1, maxWidth: 4, minHeight: 1, maxHeight: 4 }] };
  const layoutLabels = { left: "Move left", right: "Move right", up: "Move up", down: "Move down", wider: "Make wider", narrower: "Make narrower", taller: "Make taller", shorter: "Make shorter", show: "Show", hide: "Hide", invalid: "Invalid layout", failed: "Layout rejected", unknown: "Layout outcome unknown" };
  const source: ObservationSource<Row> = async function* (signal) {
    yield [{ label: "authorized equipment" }];
    await new Promise<void>((resolve) => signal.addEventListener("abort", () => resolve(), { once: true }));
  };
  const observation = createObservation({
    scope: { kind: "principal", principal: { authority: "consumer", kind: "human", subject: "alice" }, generation: 0 },
    source, clone: (row: Row) => ({ ...row }),
    measure: (rows) => new TextEncoder().encode(JSON.stringify(rows)).byteLength,
    limits: { maxRows: 10, maxBytes: 1024 },
    retry: { maxAttempts: 1, maxElapsedMs: 1000, delayMs: 0, clock: () => performance.now() },
    classifyError: () => ({ kind: "denied", code: "Denied" }),
    onAuthorityLost: () => { lost++; },
  });
  onMount(() => {
    const unsubscribe = observation.subscribe((state) => { observationState = state; });
    observation.start();
    return () => { unsubscribe(); observation.dispose(); requests.dispose(); releaseOld?.(); };
  });
  function deny() {
    observation.rebind({ kind: "principal", principal: { authority: "consumer", kind: "human", subject: "alice" }, generation: ++generation }, async function* () { throw "denied"; });
  }
  async function slowPreview() {
    await requests.run("old", () => new Promise<string>((resolve) => { releaseOld = () => resolve("obsolete preview"); }));
  }
</script>

<main class="mx-auto max-w-5xl p-4">
  <h1>Equipment inspection workspace</h1>
  <Button bind:ref={opener} onclick={() => { open = true; }}>Inspect equipment</Button>
  <Button bind:ref={fallback}>Equipment summary</Button>
  <Button onclick={() => { selection.toggle("pump-1"); selected = selection.selected; }}>Select pump</Button>
  <Button onclick={() => { selection.toggle("valve-1"); selected = selection.selected; }}>Select valve</Button>
  <output aria-label="Selected equipment">{selected.join(",")}</output>
  <Button onclick={slowPreview}>Start slow preview</Button>
  <Button onclick={() => requests.run("current", async () => "current preview")}>Load current preview</Button>
  <Button onclick={() => releaseOld?.()}>Release old preview</Button>
  <output aria-label="Preview">{latest.value ?? latest.phase}</output>
  <Button onclick={deny}>Revoke observation</Button>
  <output aria-label="Observation phase">{observationState?.phase ?? "idle"}</output>
  <output aria-label="Observed equipment">{observationState?.rows.map(row => row.label).join(",") ?? ""}</output>
  <output aria-label="Authority losses">{lost}</output>
  <section aria-label="Installed public workspace compositions" class="min-w-0 space-y-4">
    <SelectionCard id="pump-1" title="Feed pump" selected={selected.includes("pump-1")} authorityToken={generation} failedLabel="Selection rejected" unknownLabel="Selection outcome unknown" onToggle={(id) => { selection.toggle(id); selected = selection.selected; return "accepted"; }}>
      <p>Equipment available for inspection</p>
      {#snippet actions()}<Button onclick={() => { open = true; }}>Review selected equipment</Button>{/snippet}
    </SelectionCard>
    <ReferencePicker kind="equipment" value={reference} label="Related equipment" authorityToken={generation} onchange={(value) => { reference = value; }} />
    <HistoryList entries={[{ id: "inspection-1", title: "Feed pump inspection", instant: 1791453600000 }]} selectedId={historyId} authorityToken={generation} locale="en-GB" empty="No inspections" label="Inspection history" messages={(key) => key === "history.selectionFailed" ? "Inspection selection failed" : "Inspection count"} onSelect={(id) => { historyId = id; }} />
    <output aria-label="Selected inspection">{historyId ?? "none"}</output>
    <LayoutControls {layout} options={layoutOptions} label="Workspace layout" itemLabel={() => "Inspection history"} labels={layoutLabels} authorityToken={generation} onChange={(_command, proposal) => { layout = proposal; return "accepted"; }} />
    <output aria-label="Confirmed history position">{layout[0].x}</output>
  </section>
  <div class="conversation-host">
    <ConversationLayout
      label="Inspection conversation"
      bodyLabel="Inspection messages"
      bind:expanded={conversationExpanded}
      expansion={{ expandLabel: "Expand inspection conversation", collapseLabel: "Collapse inspection conversation" }}
    >
      {#snippet header()}<h2>Equipment conversation</h2>{/snippet}
      {#snippet history()}
        <details>
          <summary>Conversation history</summary>
          <ol>{#each Array(30) as _, index}<li>Saved inspection {index + 1}: equipment maintenance history</li>{/each}</ol>
        </details>
      {/snippet}
      {#snippet body()}
        <ol>{#each Array(30) as _, index}<li class="conversation-message">Inspection message {index + 1}. {"Equipment status and maintenance observations. ".repeat(8)}</li>{/each}</ol>
      {/snippet}
      {#snippet footer()}
        <form class="conversation-composer" onsubmit={(event) => { event.preventDefault(); submitConversation(); }}>
          <Label for="conversation-message">Conversation message</Label>
          <Textarea id="conversation-message" class="conversation-draft" bind:value={conversationDraft} onkeydown={(event) => {
            if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
              event.preventDefault();
              submitConversation();
            }
          }} />
          <Button type="submit" disabled={!conversationDraft.trim()}>Send conversation message</Button>
          <output aria-label="Conversation submissions">{conversationSubmissions}</output>
        </form>
      {/snippet}
    </ConversationLayout>
  </div>
  <ResponsiveDetails bind:open title="Equipment inspection" description="Review equipment and preserve your inspection draft." closeLabel="Close equipment inspection" {opener} fallbackFocus={() => fallback?.focus()}>
    <Label for="equipment-draft">Inspection notes</Label>
    <Input id="equipment-draft" bind:value={draft} />
  </ResponsiveDetails>
</main>

<style>
  .conversation-host { height: min(75dvh, 640px); min-width: 0; margin-top: 24px; display: flex; justify-content: center; }
  .conversation-composer { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 6px; }
  .conversation-composer :global(label) { grid-column: 1 / -1; }
  .conversation-composer :global(.conversation-draft) { field-sizing: fixed; height: 72px; min-height: 72px; max-height: 72px; overflow: auto; }
  .conversation-message { padding: 12px; overflow-wrap: anywhere; border-bottom: 1px solid; }
</style>
