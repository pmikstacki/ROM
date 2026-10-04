<script lang="ts">
  import { untrack } from "svelte";
  import { MediaQuery } from "svelte/reactivity";
  import { INSPECTOR_MEDIA_QUERY } from "./inspector-layout.ts";
  import { preserveEditor } from "./preserve-editor.ts";
  import type {
    ApplicationController,
    ApplicationState,
  } from "./controller.ts";
  import type { ResourceDescriptor } from "../client/types.ts";
  import type { FilterDraft } from "../filters/types.ts";
  import { draftFromQuery, queryFromDraft } from "../filters/translation.ts";
  import { stringifyWire } from "../client/codec.ts";
  import ResourceTable from "../resources/ResourceTable.svelte";
  import ResourceForm from "../resources/ResourceForm.svelte";
  import FilterPanel from "../resources/FilterPanel.svelte";
  import ResourceDetails from "./ResourceDetails.svelte";
  import ResponsiveInspector from "./ResponsiveInspector.svelte";
  import * as Popover from "../components/ui/popover/index.js";
  import * as Tabs from "../components/ui/tabs/index.js";
  import { Button } from "../components/ui/button/index.js";
  import { Badge } from "../components/ui/badge/index.js";
  import { Input } from "../components/ui/input/index.js";
  import FilterIcon from "@lucide/svelte/icons/list-filter";
  import PanelIcon from "@lucide/svelte/icons/panel-right";
  import PlusIcon from "@lucide/svelte/icons/plus";
  import RefreshIcon from "@lucide/svelte/icons/rotate-cw";
  import ObserveIcon from "@lucide/svelte/icons/radio";
  import StopIcon from "@lucide/svelte/icons/radio-tower";
  import FirstPageIcon from "@lucide/svelte/icons/chevrons-left";
  import PreviousPageIcon from "@lucide/svelte/icons/chevron-left";
  import NextPageIcon from "@lucide/svelte/icons/chevron-right";
  let {
    controller,
    snapshot,
    descriptor,
  }: {
    controller: ApplicationController;
    snapshot: ApplicationState;
    descriptor: ResourceDescriptor;
  } = $props();
  let create = $state(false),
    id = $state("");
  let draft = $state.raw<FilterDraft>(
    untrack(() => draftFromQuery(descriptor, snapshot.query)),
  );
  const definition = untrack(() =>
    stringifyWire(
      descriptor as unknown as import("../client/types.ts").WireValue,
    ),
  );
  let acceptedDefinition = $state(definition);
  const mobile = new MediaQuery(INSPECTOR_MEDIA_QUERY);
  let editorEpoch = $state(0),
    quickOpen = $state(false),
    inspectorOpen = $state(untrack(() => !mobile.current)),
    tab = $state(untrack(() => (snapshot.selected ? "details" : "filters")));
  let editorInvalid = $state(""),
    error = $state("");
  const changedDefinition = $derived(
    acceptedDefinition !==
      stringifyWire(
        descriptor as unknown as import("../client/types.ts").WireValue,
      ),
  );
  let quickTarget = $state<HTMLDivElement | null>(null),
    fullTarget = $state<HTMLDivElement | null>(null);
  let inspectorTrigger = $state<HTMLButtonElement | null>(null);
  const blocked = $derived(
    snapshot.busy ||
      snapshot.pending?.state === "unknown" ||
      snapshot.pending?.state === "pending",
  );
  const invalid = $derived(
    editorInvalid ||
      (changedDefinition
        ? "Resource definition changed. Discard edits before applying filters."
        : ""),
  );
  const edited = $derived.by(() => {
    if (editorInvalid) return true;
    try {
      return (
        stringifyWire(
          queryFromDraft(
            descriptor,
            draft,
          ) as unknown as import("../client/types.ts").WireValue,
        ) !==
        stringifyWire(
          queryFromDraft(
            descriptor,
            draftFromQuery(descriptor, snapshot.query),
          ) as unknown as import("../client/types.ts").WireValue,
        )
      );
    } catch {
      return true;
    }
  });
  const applied = $derived([
    ...(snapshot.query.filters ?? []).map(
      (item) =>
        `${item.field} = ${item.absent ? "absent" : stringifyWire(item.value)}`,
    ),
    ...(snapshot.query.comparisons ?? []).map(
      (item) =>
        `${item.field} ${item.op} ${item.absent ? "absent" : stringifyWire(item.value)}`,
    ),
    ...(snapshot.query.order ?? []).map(
      (item) => `${item.field} ${item.direction}`,
    ),
  ]);
  function discard() {
    try {
      draft = draftFromQuery(descriptor, snapshot.query);
      error = "";
    } catch {
      draft = {
        rules: { glue: "and", rules: [] },
        order: [],
        limit: String(snapshot.query.limit ?? 50),
      };
      error =
        "The current query uses unavailable fields. The draft was reset. Apply filters to load Resources.";
    }
    acceptedDefinition = stringifyWire(
      descriptor as unknown as import("../client/types.ts").WireValue,
    );
    editorInvalid = "";
    editorEpoch++;
  }
  function apply() {
    if (blocked || invalid) return;
    try {
      const query = queryFromDraft(descriptor, draft);
      error = "";
      void controller.applyQuery(query);
      quickOpen = false;
    } catch (problem) {
      error = problem instanceof Error ? problem.message : "Invalid filters.";
    }
  }
  function fullFilters() {
    quickOpen = false;
    inspectorOpen = true;
    tab = "filters";
  }
  function openInspector() {
    inspectorOpen = true;
    tab = snapshot.selected ? "details" : "filters";
  }
</script>

<div class="flex flex-wrap items-center justify-between gap-3">
  <div>
    <h1 class="text-xl font-semibold tracking-tight">{descriptor.kind}</h1>
    <p class="text-xs text-muted-foreground">
      Manage Resources and observe committed changes.
    </p>
  </div>
  <Button
    size="sm"
    class="max-lg:size-9"
    disabled={blocked}
    aria-label="Create Resource"
    title="Create Resource"
    onclick={() => (create = !create)}
    ><PlusIcon /><span class="hidden lg:inline">Create Resource</span></Button
  >
</div>
<div class="flex flex-wrap items-center gap-2 rounded-lg border bg-card p-2">
  <div
    class="flex shrink-0 items-center gap-1"
    role="group"
    aria-label="Filter and details controls"
  >
    <Popover.Root bind:open={quickOpen}>
      <Popover.Trigger>
        {#snippet child({ props })}<Button
            {...props}
            variant="outline"
            size="sm"
            class="relative max-lg:size-9"
            aria-label="Quick filters"
            title="Quick filters"
            ><FilterIcon /><span class="hidden lg:inline">Filters</span
            >{#if applied.length}<Badge
                variant="secondary"
                class="max-lg:absolute max-lg:-right-1 max-lg:-top-1 max-lg:min-w-4 max-lg:px-1 max-lg:text-[10px]"
                >{applied.length}</Badge
              >{/if}</Button
          >{/snippet}
      </Popover.Trigger>
      <Popover.Content
        role="dialog"
        forceMount
        hidden={!quickOpen}
        aria-label="Quick filters"
        align="start"
        class="max-h-[min(80vh,42rem)] w-[min(24rem,calc(100vw-2rem))] overflow-y-auto"
      >
        <div class="flex items-center justify-between gap-2">
          <h2 class="font-semibold">Quick filters</h2>
          <Button variant="ghost" size="sm" onclick={fullFilters}
            >Open full filters</Button
          >
        </div>
        <div bind:this={quickTarget}></div>
      </Popover.Content>
    </Popover.Root>
    <Button
      variant="ghost"
      size="sm"
      class="max-lg:size-9"
      bind:ref={inspectorTrigger}
      aria-label="Open details panel"
      title="Open details panel"
      onclick={openInspector}
      ><PanelIcon /><span class="hidden lg:inline">Details panel</span></Button
    >
  </div>
  <div
    role="group"
    aria-label="Applied query"
    class="order-last flex min-w-0 basis-full flex-wrap gap-1.5 lg:order-none lg:basis-auto lg:flex-1"
  >
    {#each applied as label}<Badge
        variant="secondary"
        class="max-w-full whitespace-normal break-all">{label}</Badge
      >{/each}
    {#if !applied.length}<span class="text-xs text-muted-foreground"
        >All authorized Resources</span
      >{/if}
  </div>
  <Button
    variant="ghost"
    size="sm"
    class="max-lg:size-9"
    aria-label="Refresh"
    title="Refresh"
    disabled={snapshot.busy}
    onclick={() => void controller.refresh()}
    ><RefreshIcon /><span class="hidden lg:inline">Refresh</span></Button
  >
  <Button
    variant="outline"
    size="sm"
    class="max-lg:size-9"
    aria-label={snapshot.live ? "Stop live query" : "Observe live query"}
    title={snapshot.live ? "Stop live query" : "Observe live query"}
    disabled={snapshot.busy}
    onclick={() =>
      snapshot.live ? controller.stopLive() : void controller.observe()}
    >{#if snapshot.live}<StopIcon />{:else}<ObserveIcon />{/if}<span
      class="hidden lg:inline"
      >{snapshot.live ? "Stop live query" : "Observe live query"}</span
    ></Button
  >
</div>
{#if snapshot.busy}<p role="status">Loading…</p>{/if}
{#if error}<p role="alert" class="text-sm text-destructive">{error}</p>{/if}
{#if create}<section
    class="space-y-4 rounded-lg border bg-card p-4"
    aria-label="Create Resource"
  >
    <label>Resource ID<Input bind:value={id} /></label><ResourceForm
      {descriptor}
      mode="create"
      submit={async (input) => {
        await controller.mutate(id, null, input);
        create = false;
      }}
    />
  </section>{/if}
<div
  class={inspectorOpen
    ? "grid items-start gap-4 xl:grid-cols-[minmax(0,1fr)_20rem]"
    : "grid gap-4"}
>
  <div class="min-w-0 space-y-3">
    <ResourceTable
      {descriptor}
      rows={snapshot.rows}
      onselect={(row) => {
        inspectorOpen = true;
        tab = "details";
        void controller.selectRow(row.key.id);
      }}
    />
    <nav
      class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground"
      aria-label="Query pages"
    >
      <span class="mr-auto"
        ><span class="inline-block">Moving page {snapshot.page}</span> · {snapshot
          .query.limit ?? 50} per page</span
      >
      <Button
        variant="outline"
        size="sm"
        class="max-lg:size-9"
        aria-label="First page"
        title="First page"
        disabled={snapshot.busy || snapshot.page === 1}
        onclick={() => void controller.firstPage()}
        ><FirstPageIcon /><span class="hidden lg:inline">First page</span
        ></Button
      >
      <Button
        variant="outline"
        size="sm"
        class="max-lg:size-9"
        aria-label="Previous page"
        title="Previous page"
        disabled={snapshot.busy || !snapshot.hasPrevious}
        onclick={() => void controller.previousPage()}
        ><PreviousPageIcon /><span class="hidden lg:inline">Previous page</span
        ></Button
      >
      <Button
        variant="outline"
        size="sm"
        class="max-lg:size-9"
        aria-label="Next page"
        title="Next page"
        disabled={snapshot.busy ||
          snapshot.rows.length < (snapshot.query.limit ?? 50)}
        onclick={() => void controller.nextPage()}
        ><NextPageIcon /><span class="hidden lg:inline">Next page</span></Button
      >
    </nav>
  </div>
  <ResponsiveInspector
    bind:open={inspectorOpen}
    title={tab === "details" ? "Edit Resource" : "Filters"}
    onCloseFocus={() => inspectorTrigger?.focus()}
  >
    <Tabs.Root bind:value={tab} class="gap-4">
      <Tabs.List class="w-full"
        ><Tabs.Trigger value="filters" class="flex-1">Filters</Tabs.Trigger
        ><Tabs.Trigger value="details" class="flex-1">Details</Tabs.Trigger
        ></Tabs.List
      >
      <Tabs.Content value="filters" hidden={tab !== "filters"}>
        <section aria-label="Filter editor" class="space-y-4">
          <div class="flex items-center justify-between gap-2">
            <h2 class="font-semibold">Filters</h2>
            {#if edited}<Badge variant="outline">Pending edits</Badge
              >{:else}<Badge variant="secondary">Applied</Badge>{/if}
          </div>
          <p class="text-xs text-muted-foreground">
            Conditions combine with AND. Changes run only when applied.
          </p>
          <div bind:this={fullTarget}></div>
          <div use:preserveEditor={quickOpen ? quickTarget : fullTarget}>
            {#key editorEpoch}<FilterPanel
                {descriptor}
                {draft}
                onchange={(next) => (draft = next)}
                disabled={blocked || changedDefinition}
                discardDisabled={blocked}
                {invalid}
                compact={quickOpen}
                onvalidity={(next) => (editorInvalid = next)}
                onapply={apply}
                ondiscard={discard}
              />{/key}
            {#if invalid}<p role="alert" class="text-sm text-destructive">
                {invalid}
              </p>{/if}
          </div>
        </section>
      </Tabs.Content>
      <Tabs.Content value="details" hidden={tab !== "details"}>
        {#if snapshot.selected && snapshot.selected.key.kind === descriptor.kind}
          {#key snapshot.selected.key.id}<ResourceDetails
              {descriptor}
              selected={snapshot.selected}
              {blocked}
              onmutate={async (expected, operation) => {
                await controller.mutate(
                  snapshot.selected!.key.id,
                  expected,
                  operation,
                );
              }}
              onreload={() =>
                void controller.selectRow(snapshot.selected!.key.id)}
            />{/key}
        {:else}<p class="text-sm text-muted-foreground">
            Select a Resource to view and edit its details.
          </p>{/if}
      </Tabs.Content>
    </Tabs.Root>
  </ResponsiveInspector>
</div>
