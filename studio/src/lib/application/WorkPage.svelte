<script lang="ts">
  import { RemoteError } from "../client/client.ts";
  import { onDestroy, onMount } from "svelte";
  import type {
    ApplicationState,
    ApplicationController,
  } from "./controller.ts";
  import type { WireObject } from "../client/types.ts";
  import { stringifyWire } from "../client/codec.ts";
  import { record } from "../client/validation.ts";
  import ResponsiveInspector from "./ResponsiveInspector.svelte";
  import InspectorToggle from "./InspectorToggle.svelte";
  import WorkDetails from "./WorkDetails.svelte";
  import WorkFilters from "./WorkFilters.svelte";
  import WorkRecovery from "./WorkRecovery.svelte";
  import * as Tabs from "../components/ui/tabs/index.js";
  import { Button } from "../components/ui/button/index.js";
  import { Badge } from "../components/ui/badge/index.js";
  import {
    workStatus,
    workDefinition,
    workQuery,
  } from "./work-presentation.ts";
  let {
    snapshot,
    controller,
    onNavigationBlockChange = () => {},
  }: {
    snapshot: ApplicationState;
    controller: ApplicationController;
    onNavigationBlockChange?: (blocked: boolean) => void;
  } = $props();
  const componentId = $props.id();
  const inspectorId = `work-inspector-${componentId}`;
  let inspectorTrigger = $state<HTMLButtonElement | null>(null);
  let inspectorOpener = $state<HTMLButtonElement | null>(null);
  let inspectorOpen = $state(false),
    tab = $state("filters");
  let records = $state.raw<WireObject[]>([]),
    selected = $state.raw<WireObject | null>(null),
    result = $state.raw<WireObject | null>(null),
    error = $state(""),
    busy = $state(false),
    loaded = $state(false),
    pending = $state.raw<WireObject | null>(null),
    confirmation = $state(false),
    evidence = $state(""),
    fresh = $state(true),
    cursor = $state<string | null>(null);
  let category = $state(""),
    statusFilter = $state(""),
    definition = $state("");
  let applied = $state.raw<WireObject>({ limit: 50 });
  let destroyed = false;
  onDestroy(() => {
    destroyed = true;
  });
  const capabilities = $derived(
    snapshot.work !== null ? record(snapshot.work) : null,
  );
  const blocked = $derived(busy || pending !== null);
  $effect(() => onNavigationBlockChange(blocked));
  const filtersEdited = $derived(
    stringifyWire(workQuery(category, statusFilter, definition)) !==
      stringifyWire(applied),
  );
  async function run(route: "list" | "read" | "control", request: WireObject) {
    busy = true;
    error = "";
    try {
      const value = await controller.inspectWork(route, request);
      if (destroyed) return;
      if (route === "list") {
        const page = record(value);
        if (!Array.isArray(page.records)) throw Error("Invalid work page.");
        records = page.records.map(record);
        cursor = page.cursor === null ? null : String(page.cursor);
        selected = null;
        confirmation = false;
        loaded = true;
      } else if (route === "read") {
        selected = record(value);
        confirmation = false;
        evidence = "";
        result = null;
        fresh = true;
      } else {
        result = record(value);
        pending = null;
        confirmation = false;
        fresh = false;
      }
    } catch (problem) {
      if (
        !destroyed &&
        route === "control" &&
        problem instanceof RemoteError &&
        (problem.category === "conflict" ||
          problem.category === "not_committed" ||
          (problem.status > 0 && problem.status < 500))
      ) {
        pending = null;
        confirmation = false;
        fresh = false;
      }
      if (!destroyed)
        error =
          problem instanceof Error ? problem.message : "Work request failed.";
    } finally {
      if (!destroyed) busy = false;
    }
  }
  async function connect() {
    if (blocked) return;
    busy = true;
    error = "";
    try {
      await controller.loadWork();
      if (destroyed) return;
      const available = controller.state.work;
      if (available !== null && record(available).inspect === true)
        await run("list", applied);
    } finally {
      if (!destroyed) busy = false;
    }
  }
  onMount(() => void connect());
  async function applyFilters() {
    if (blocked) return;
    applied = workQuery(category, statusFilter, definition);
    await run("list", applied);
  }
  async function control(operation: "Retry" | "Reconcile") {
    if (!selected || !confirmation || pending || busy || !fresh) return;
    const request: WireObject = {
      handle: selected.handle,
      expected: selected.version,
      key: crypto.randomUUID(),
      retry_epoch: 0n,
      operation:
        operation === "Retry"
          ? "Retry"
          : { Reconcile: { evidence_ref: evidence || null } },
    };
    pending = request;
    await run("control", request);
  }
</script>

<div class="space-y-4">
  <div>
    <h1 class="text-xl font-semibold tracking-tight">Work</h1>
    <p class="text-sm text-muted-foreground">
      Inspect current work status and recover work when authorized.
    </p>
  </div>
  <div class="flex flex-wrap items-center gap-2 rounded-lg border bg-card p-2">
    <Button
      variant="outline"
      size="sm"
      disabled={blocked}
      onclick={() => void connect()}>Refresh work</Button
    >
    {#if capabilities?.inspect === true}<span
        class="text-xs text-muted-foreground"
        >{records.length} on this page</span
      >{/if}
    {#if applied.category || applied.state || applied.definition}<Badge
        variant="secondary">Filtered</Badge
      >{/if}
    <div class="ml-auto">
      <InspectorToggle
        bind:ref={inspectorTrigger}
        open={inspectorOpen}
        controls={inspectorId}
        onclick={() => {
          inspectorOpener = inspectorTrigger;
          inspectorOpen = !inspectorOpen;
        }}
      />
    </div>
  </div>
  {#if busy}<p role="status" class="text-sm text-muted-foreground">
      Loading work…
    </p>{/if}
  {#if error}<p role="alert" class="text-sm text-destructive">{error}</p>{/if}
  {#if result}<p role="status" class="text-sm">
      Recovery result: {String(result.outcome)}{result.replayed
        ? " · Replayed receipt"
        : ""}. Inspect the item again before another recovery action.
    </p>{/if}
  <div
    class={inspectorOpen
      ? "grid items-start gap-4 xl:grid-cols-[minmax(0,1fr)_20rem]"
      : "grid gap-4"}
  >
    <div class="min-w-0 space-y-3">
      {#if capabilities?.inspect === false}<p
          class="text-sm text-muted-foreground"
        >
          Work inspection is unavailable for this session.
        </p>
      {:else if loaded && !records.length}<p
          class="rounded-lg border p-4 text-sm text-muted-foreground"
        >
          No work matches the applied filters.
        </p>{/if}
      <ul class="space-y-2" aria-label="Work items">
        {#each records as item (String(item.handle))}<li
            class="flex min-w-0 items-start gap-3 rounded-lg border bg-card p-3"
          >
            <div class="min-w-0 flex-1 space-y-1">
              <p class="break-words font-medium">{workDefinition(item)}</p>
              <div class="flex flex-wrap items-center gap-2 text-xs">
                <Badge variant="outline">{workStatus(item.state)}</Badge><span
                  class="text-muted-foreground">{String(item.category)}</span
                >
              </div>
              <p class="text-xs text-muted-foreground">
                {String(item.attempts)} attempts · Due value {String(item.due)}
              </p>
              <p
                class="truncate font-mono text-xs text-muted-foreground"
                title={String(item.handle)}
              >
                {String(item.handle)}
              </p>
            </div>
            <Button
              size="sm"
              variant="outline"
              disabled={blocked}
              aria-label={`Inspect ${workDefinition(item)}`}
              onclick={(event) => {
                if (event.currentTarget instanceof HTMLButtonElement) {
                  inspectorOpener = event.currentTarget;
                  inspectorOpener.focus();
                }
                inspectorOpen = true;
                tab = "details";
                void run("read", { handle: item.handle });
              }}>Inspect</Button
            >
          </li>{/each}
      </ul>
      {#if cursor}<Button
          variant="outline"
          disabled={blocked}
          onclick={() => void run("list", { ...applied, cursor })}
          >Next work page</Button
        >{/if}
    </div>
    <ResponsiveInspector
      id={inspectorId}
      bind:open={inspectorOpen}
      title="Work tools"
      label="Work tools"
      description="Filters and selected work details."
      onCloseFocus={() => {
        const opener =
          inspectorOpener?.isConnected && !inspectorOpener.disabled
            ? inspectorOpener
            : inspectorTrigger;
        opener?.focus();
      }}
    >
      <Tabs.Root bind:value={tab} class="gap-4">
        <Tabs.List class="w-full"
          ><Tabs.Trigger value="filters" class="flex-1">Filters</Tabs.Trigger
          ><Tabs.Trigger value="details" class="flex-1">Details</Tabs.Trigger
          ></Tabs.List
        >
        <Tabs.Content value="filters" hidden={tab !== "filters"}>
          <WorkFilters
            bind:category
            bind:statusFilter
            bind:definition
            edited={filtersEdited}
            disabled={blocked}
            canInspect={capabilities?.inspect === true}
            onapply={() => void applyFilters()}
          />
        </Tabs.Content>
        <Tabs.Content value="details" hidden={tab !== "details"}>
          {#if selected}<WorkDetails item={selected} />{:else}<p
              class="text-sm text-muted-foreground"
            >
              Select a work item to inspect its current status.
            </p>{/if}
          <WorkRecovery
            bind:confirmation
            bind:evidence
            hasSelection={selected !== null}
            canRetry={capabilities?.retry === true}
            canReconcile={capabilities?.reconcile === true}
            {fresh}
            {blocked}
            {busy}
            pending={pending !== null}
            oncontrol={(operation) => void control(operation)}
            onretry={() => {
              if (pending) void run("control", pending);
            }}
          />
        </Tabs.Content>
      </Tabs.Root>
    </ResponsiveInspector>
  </div>
</div>
