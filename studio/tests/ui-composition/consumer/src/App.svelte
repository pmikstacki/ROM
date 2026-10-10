<script lang="ts">
  import { onMount, untrack } from "svelte";
  import {
    Button,
    Label,
    NativeSelect,
    NativeSelectOption,
  } from "rom-studio/controls";
  import {
    SelectionCard,
    ResponsiveDetails,
    HistoryList,
    LayoutControls,
  } from "rom-studio/ui/components";
  import type { WireValue } from "rom-studio/client";
  import { createPortalHost } from "./host.ts";
  import type { PortalState } from "./host.ts";
  import { field, layoutLabels, panelLabels, settingsLayout } from "./model.ts";
  import InspectionEditor from "./InspectionEditor.svelte";
  import PublicGuides from "./PublicGuides.svelte";
  let portal = $state.raw<PortalState>(null!);
  const host = createPortalHost(
    (next) => (portal = next),
    new URLSearchParams(location.search).get("workspace") === "public"
      ? "public-guest"
      : "alice",
  );
  portal = host.state;
  let role = $state("technician"),
    open = $state(false),
    notes = $state(""),
    date = $state(""),
    equipment = $state<WireValue>("");
  let opener = $state<HTMLElement | null>(null),
    fallback = $state<HTMLButtonElement | HTMLAnchorElement | null>(null);
  const downloadUrls = new Set<string>();
  const inspection = $derived(
    portal.rows.inspections.find((row) => row.key.id === "check-1"),
  );
  const settings = $derived(
    portal.rows["portal-settings"].find((row) => row.key.id === "workspace"),
  );
  const layout = $derived(settingsLayout(settings));
  const settingsRecovery = $derived(
    portal.recovery["portal-settings/workspace"],
  );
  const workRecovery = $derived(portal.recovery["work-orders/repair-1"]);
  const authorityGeneration = $derived(portal.generation);
  function panelPlacement(id: string) {
    const item = layout?.layout.find((item) => item.id === id);
    return item
      ? `--panel-column: ${item.x + 1}; --panel-width: ${item.width}; --panel-row: ${item.y + 1}; --panel-height: ${item.height}; ${item.visible ? "" : "display: none"}`
      : "--panel-column: 1; --panel-width: var(--workspace-columns); --panel-row: auto; --panel-height: 1";
  }
  $effect(() => {
    authorityGeneration;
    untrack(() => {
      open = false;
      notes = "";
      date = "";
      equipment = "";
      opener = null;
    });
  });
  onMount(() => {
    void host.start().catch(() => {});
    const visibility = () =>
      host.visible(document.visibilityState !== "hidden");
    document.addEventListener("visibilitychange", visibility);
    return () => {
      document.removeEventListener("visibilitychange", visibility);
      host.dispose();
      for (const url of downloadUrls) URL.revokeObjectURL(url);
      downloadUrls.clear();
    };
  });
  function inspect(event: MouseEvent, id: string) {
    opener = event.currentTarget as HTMLElement;
    notes = field(inspection, "notes");
    date = field(inspection, "inspected_at");
    equipment = id;
    open = true;
  }
  async function switchSubject(subject: PortalState["subject"]) {
    open = false;
    await host.switchSubject(subject);
  }
  function downloadCaptured() {
    if (portal.exportPhase !== "ready" || !portal.exportText) return;
    const ticket = portal.generation;
    const url = URL.createObjectURL(
      new Blob([portal.exportText], { type: "application/json" }),
    );
    downloadUrls.add(url);
    if (portal.generation !== ticket) {
      URL.revokeObjectURL(url);
      downloadUrls.delete(url);
      return;
    }
    const link = document.createElement("a");
    link.href = url;
    link.download = "inspection-check-1.json";
    link.click();
    setTimeout(() => {
      URL.revokeObjectURL(url);
      downloadUrls.delete(url);
    }, 1000);
  }
</script>

<main class="mx-auto grid max-w-6xl min-w-0 gap-5 p-4">
  <header class="grid min-w-0 gap-2">
    <h1 class="text-2xl font-semibold">Maintenance workspace</h1>
    <p class="text-sm text-muted-foreground">
      {portal.subject === "public-guest"
        ? "Select public guides, compute their nominal values, and export the selected snapshot."
        : "Review equipment, record an inspection, and arrange your workspace."}
    </p>
    <div class="flex flex-wrap items-center gap-2">
      {#if portal.subject !== "public-guest"}<Label for="role"
          >Workspace role</Label
        >
        <NativeSelect id="role" bind:value={role} class="w-auto">
          <NativeSelectOption value="technician">Technician</NativeSelectOption>
          <NativeSelectOption value="dispatcher">Dispatcher</NativeSelectOption>
        </NativeSelect>
      {/if}
      <Button
        variant="outline"
        onclick={() => void switchSubject("public-guest")}
        >Use public guest access</Button
      >
      <Button variant="outline" onclick={() => void switchSubject("alice")}
        >Use Alice fixture identity</Button
      >
      <Button variant="outline" onclick={() => void switchSubject("bob")}
        >Use Bob fixture identity</Button
      >
      <Button variant="outline" onclick={() => void switchSubject("guest")}
        >Use unverified fixture identity</Button
      >
    </div>
    <p class="text-xs text-muted-foreground">
      Fixture credentials exercise the test server. Workspace roles do not grant
      permission.
    </p>
    <div class="flex flex-wrap gap-4 text-xs">
      <span
        >Principal: <output aria-label="Current principal"
          >{portal.subject}</output
        ></span
      >
      {#if portal.subject !== "public-guest"}<span
          >Equipment: <output aria-label="Equipment observation"
            >{portal.phases.equipment}</output
          ></span
        >
        <span
          >Inspection: <output aria-label="Inspection observation"
            >{portal.phases.inspections}</output
          ></span
        >
      {/if}
    </div>
  </header>
  {#if portal.subject === "public-guest"}
    {#key portal.generation}<PublicGuides
        generation={portal.generation}
      />{/key}
  {:else}
    {#if portal.error}<p role="status">{portal.error}</p>{/if}
    <div
      class="grid min-w-0 gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(20rem,0.8fr)]"
    >
      <div class="grid min-w-0 content-start gap-5">
        <output aria-label="History visibility"
          >{settings?.value?.show_history === false
            ? "hidden"
            : "visible"}</output
        >
        <div
          class="workspace-grid"
          style={`--workspace-columns: ${layout?.options.columns ?? 1}`}
        >
          <section
            aria-label="Equipment"
            class="workspace-panel grid min-w-0 content-start gap-3"
            style={panelPlacement("equipment")}
          >
            <h2 class="font-semibold">Equipment</h2>
            {#each portal.rows.equipment as row (row.key.id)}
              <SelectionCard
                id={row.key.id}
                title={field(row, "title")}
                selected={portal.selected.includes(row.key.id)}
                authorityToken={portal.generation}
                failedLabel="Selection unavailable"
                unknownLabel="Selection outcome unknown"
                onToggle={(id) => host.toggle(id)}
              >
                <span class="text-sm">Equipment ID: {row.key.id}</span>
                {#snippet actions()}<Button
                    variant="outline"
                    onclick={(event) => inspect(event, row.key.id)}
                    >Inspect {field(row, "title")}</Button
                  >{/snippet}
              </SelectionCard>
            {:else}<p>No equipment is currently disclosed.</p>{/each}
            <output aria-label="Selected equipment"
              >{portal.selected.join(", ") || "none"}</output
            >
          </section>
          <section
            class="workspace-panel grid min-w-0 content-start gap-2"
            aria-label="Confirmed inspection"
            style={panelPlacement("inspection")}
          >
            <h2 class="font-semibold">Confirmed inspection</h2>
            <output aria-label="Confirmed inspection date"
              >{field(inspection, "inspected_at") || "none"}</output
            >
            <Button
              disabled={!inspection || portal.exportPhase === "pending"}
              onclick={() => void host.exportInspection()}
              >Export inspection JSON</Button
            >
            {#if portal.exportPhase === "ready"}<Button
                variant="outline"
                onclick={downloadCaptured}>Download captured JSON</Button
              >{/if}
            <output
              class="block min-w-0 break-all text-xs"
              aria-label="Export result">{portal.exportText || "none"}</output
            >
          </section>
          <section
            class="workspace-panel grid min-w-0 content-start gap-2"
            aria-label="Work orders"
            style={panelPlacement("work")}
          >
            <h2 class="font-semibold">Work orders</h2>
            {#each portal.rows["work-orders"] as row (row.key.id)}
              <p>
                {field(row, "title")} · {row.value?.completed
                  ? "Complete"
                  : "Open"}
              </p>
              <Button
                disabled={portal.subject !== "alice" ||
                  !!row.value?.completed ||
                  !!portal.stored["work-orders/repair-1"] ||
                  workRecovery?.hasUnresolvedIntent}
                onclick={() => void host.completeWork()}
                >Complete work order</Button
              >
            {/each}
            <output aria-label="Work save"
              >{workRecovery?.phase ?? "idle"}</output
            >
            {#if portal.subject === "alice" && portal.rows["work-orders"].some((row) => row.key.id === "repair-1")}
              {#if portal.stored["work-orders/repair-1"]}<Button
                  onclick={() => void host.restore("work-orders", "repair-1")}
                  >Restore pending work order</Button
                >{/if}
              {#if workRecovery?.hasUnresolvedIntent && workRecovery.phase !== "submitting"}<Button
                  onclick={() => void host.retry("work-orders", "repair-1")}
                  >Retry original work order</Button
                >{/if}
            {/if}
          </section>
          <section
            class="workspace-panel grid min-w-0 content-start gap-2"
            aria-label="History panel"
            style={panelPlacement("history")}
          >
            {#if settings?.value?.show_history !== false}
              <Button
                bind:ref={fallback}
                variant="outline"
                disabled={!inspection}
                onclick={() => void host.history()}
                >Refresh inspection history</Button
              >
              <HistoryList
                entries={portal.history}
                selectedId={portal.selectedHistory}
                authorityToken={portal.generation}
                label="Inspection history"
                locale="en-GB"
                timeZone="UTC"
                empty="No inspection history loaded"
                messages={(key, values) =>
                  key === "history.count"
                    ? `${values?.formattedCount} inspections`
                    : "History unavailable"}
                onSelect={(id) => host.selectHistory(id)}
              />
              <output aria-label="Selected history revision"
                >{portal.selectedHistoryRevision || "none"}</output
              >
            {/if}
          </section>
        </div>
        {#if role === "dispatcher"}
          {#if layout}
            <LayoutControls
              layout={layout.layout}
              options={layout.options}
              label="Workspace layout"
              itemLabel={(id) => panelLabels[id] ?? id}
              labels={layoutLabels}
              authorityToken={portal.generation}
              disabled={portal.subject !== "alice" ||
                !!portal.stored["portal-settings/workspace"] ||
                settingsRecovery?.hasUnresolvedIntent}
              onChange={(_command, proposal) => host.saveLayout(proposal)}
            />
            <output aria-label="Confirmed history position"
              >{layout.layout.find((item) => item.id === "history")?.x ??
                "absent"}</output
            >
          {:else}<p role="status">
              No valid authorized Settings layout is available.
            </p>{/if}
          <output aria-label="Settings save"
            >{settingsRecovery?.phase ?? "idle"}</output
          >
          {#if portal.stored["portal-settings/workspace"]}<Button
              onclick={() => void host.restore("portal-settings", "workspace")}
              >Restore pending Settings</Button
            >{/if}
          {#if settingsRecovery?.hasUnresolvedIntent && settingsRecovery.phase !== "submitting"}<Button
              onclick={() => void host.retry("portal-settings", "workspace")}
              >Retry original Settings</Button
            >{/if}
        {/if}
      </div>
      <ResponsiveDetails
        bind:open
        title="Inspection details"
        description="Record an inspection of the selected equipment."
        closeLabel="Close inspection details"
        {opener}
        fallbackFocus={() => fallback?.focus()}
      >
        <InspectionEditor
          {host}
          state={portal}
          bind:notes
          bind:date
          bind:equipment
        />
      </ResponsiveDetails>
    </div>
  {/if}
</main>

<style>
  .workspace-grid {
    display: grid;
    min-width: 0;
    gap: 1.25rem;
    grid-template-columns: repeat(var(--workspace-columns), minmax(0, 1fr));
    grid-auto-rows: minmax(16rem, auto);
  }
  .workspace-panel {
    grid-column: var(--panel-column) / span var(--panel-width);
    grid-row: var(--panel-row) / span var(--panel-height);
    overflow-wrap: anywhere;
  }
  @media (max-width: 639px) {
    .workspace-grid {
      grid-template-columns: minmax(0, 1fr);
      grid-auto-rows: auto;
    }
    .workspace-panel {
      grid-column: 1;
      grid-row: auto;
    }
  }
  :global(body) {
    margin: 0;
  }
  :global(output) {
    overflow-wrap: anywhere;
  }
</style>
