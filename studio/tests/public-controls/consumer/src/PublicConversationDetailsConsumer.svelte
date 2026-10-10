<script lang="ts">
  import { onDestroy } from "svelte";
  import { ResponsiveDetails, ConversationLayout, HistoryList } from "rom-studio/ui/components";
  import type { HistoryEntry } from "rom-studio/ui/components";
  import { createLatestRequest } from "rom-studio/ui";
  import type { LatestRequestState } from "rom-studio/ui";
  import { Button, Label, Textarea } from "rom-studio/controls";

  let open = $state(false);
  let expanded = $state(false);
  let opener = $state<HTMLButtonElement | HTMLAnchorElement | null>(null);
  let fallback = $state<HTMLButtonElement | HTMLAnchorElement | null>(null);
  let draft = $state("");
  let submissions = $state(0);
  let authority = $state(0);
  let selected = $state<string | null>(null);
  let pending = $state<LatestRequestState<string>>({ phase: "idle", identity: null, value: null, code: null });
  let held: (() => void) | undefined;
  let holdNext = false;
  let empty = $state(false);
  const entries = $derived<readonly HistoryEntry[]>(empty ? [] : Array.from({ length: 40 }, (_, index) => ({
    id: `history-${authority}-${index}`,
    title: `Saved conversation ${index + 1}: ${"bounded readable history ".repeat(3)}`,
    instant: 1791453600000 + index * 1000,
  })));
  const latest = createLatestRequest({
    clone: (value: string) => value,
    classifyError: () => "Unavailable",
    onState: (state: LatestRequestState<string>) => {
      pending = state;
      if (state.phase === "ready") selected = state.value;
      else if (state.phase === "idle") selected = null;
    },
  });
  onDestroy(() => { latest.dispose(); held?.(); });
  function choose(id: string) {
    const delay = holdNext;
    holdNext = false;
    return latest.run(`${authority}:${id}`, () => delay
      ? new Promise<string>((resolve) => { held = () => resolve(id); })
      : Promise.resolve(id));
  }
  function changeAuthority() {
    authority++;
    latest.clear();
  }
  function send() {
    if (!draft.trim()) return;
    submissions++;
    draft = "";
  }
</script>

<main class="workspace">
  <h1>Public conversation and details composition</h1>
  <Button bind:ref={opener} onclick={() => { open = true; }}>Open conversation details</Button>
  <Button bind:ref={fallback}>Conversation summary</Button>
  <ResponsiveDetails bind:open title="Conversation details" description="Inspect saved history and keep your draft." closeLabel="Close conversation details" {opener} fallbackFocus={() => fallback?.focus()}>
    <div class="conversation-host">
      <ConversationLayout label="Combined conversation" bodyLabel="Combined messages" bind:expanded expansion={{ expandLabel: "Expand conversation", collapseLabel: "Collapse conversation" }}>
        {#snippet header()}
          <div class="header-controls">
            <h2>Conversation</h2>
            <Button size="sm" onclick={() => { holdNext = true; }}>Hold next history</Button>
            <Button size="sm" onclick={changeAuthority}>Change authority</Button>
            <Button size="sm" onclick={() => { held?.(); held = undefined; }}>Complete held history</Button>
            <Button size="sm" onclick={() => { empty = !empty; }}>Toggle empty history</Button>
          </div>
        {/snippet}
        {#snippet history()}
          <HistoryList {entries} selectedId={selected} onSelect={choose} authorityToken={authority} label="Saved conversations" empty="No saved conversations" locale="en-GB" messages={(key) => key === "history.selectionFailed" ? "History selection failed" : "History count"} />
        {/snippet}
        {#snippet body()}
          <output aria-label="Selected history">{selected ?? "none"}</output>
          <output aria-label="Pending history identity">{pending.identity ?? "none"}</output>
          <output aria-label="History request phase">{pending.phase}</output>
          <ol>{#each Array(40) as _, index}<li class="message">Message {index + 1}. {"Readable conversation message. ".repeat(10)}</li>{/each}</ol>
        {/snippet}
        {#snippet footer()}
          <form onsubmit={(event) => { event.preventDefault(); send(); }}>
            <Label for="combined-draft">Message draft</Label>
            <Textarea id="combined-draft" class="draft" bind:value={draft} onkeydown={(event) => {
              if (event.key === "Enter" && !event.shiftKey && !event.isComposing) { event.preventDefault(); send(); }
            }} />
            <Button type="submit" disabled={!draft.trim()}>Send message</Button>
            <output aria-label="Submitted messages">{submissions}</output>
          </form>
        {/snippet}
      </ConversationLayout>
    </div>
  </ResponsiveDetails>
</main>

<style>
  .workspace { max-width: 60rem; margin-inline: auto; padding: 1rem; min-width: 0; }
  .conversation-host { height: min(650px, calc(100dvh - 120px)); min-height: 0; min-width: 0; display: flex; justify-content: center; }
  .header-controls { display: flex; flex-wrap: wrap; gap: .25rem; align-items: center; }
  .header-controls h2 { flex-basis: 100%; }
  form { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: .25rem; align-items: center; }
  form :global(label) { grid-column: 1 / -1; }
  form :global(.draft) { field-sizing: fixed; height: 64px; min-height: 64px; max-height: 64px; overflow: auto; }
  .message { padding-block: .5rem; overflow-wrap: anywhere; }
</style>
