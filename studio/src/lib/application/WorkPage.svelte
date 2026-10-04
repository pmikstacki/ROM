<script lang="ts">
  import { RemoteError } from "../client/client.ts";
  import { onDestroy } from "svelte";
  import type {
    ApplicationState,
    ApplicationController,
  } from "./controller.ts";
  import type { WireObject, WireValue } from "../client/types.ts";
  import { stringifyWire } from "../client/codec.ts";
  import { record } from "../client/validation.ts";
  import CheckboxAdapter from "../renderers/CheckboxAdapter.svelte";
  import { Input } from "../components/ui/input/index.js";
  import { Button } from "../components/ui/button/index.js";
  let {
    snapshot,
    controller,
  }: { snapshot: ApplicationState; controller: ApplicationController } =
    $props();
  let records = $state.raw<WireObject[]>([]),
    selected = $state.raw<WireObject | null>(null),
    result = $state.raw<WireValue>(null),
    error = $state(""),
    busy = $state(false),
    pending = $state.raw<WireObject | null>(null),
    confirmation = $state(false),
    evidence = $state("");
  let destroyed = false;
  onDestroy(() => (destroyed = true));
  const capabilities = $derived(
    snapshot.work !== null ? record(snapshot.work) : null,
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
        selected = null;
      } else if (route === "read") {
        selected = record(value);
        confirmation = false;
      } else {
        result = value;
        pending = null;
      }
    } catch (problem) {
      if (
        !destroyed &&
        route === "control" &&
        problem instanceof RemoteError &&
        (problem.category === "conflict" ||
          problem.category === "not_committed" ||
          (problem.status > 0 && problem.status < 500))
      )
        pending = null;
      if (!destroyed)
        error =
          problem instanceof Error ? problem.message : "Work request failed.";
    } finally {
      if (!destroyed) busy = false;
    }
  }
  async function control(operation: "Retry" | "Reconcile") {
    if (!selected || !confirmation || pending) return;
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

<h1 class="text-xl font-semibold tracking-tight">Work</h1>
<p>
  Inspect and recover durable work through ROM's authorized operator contract.
</p>
<Button disabled={busy} onclick={() => void controller.loadWork()}
  >Load work capabilities</Button
>
{#if capabilities}<p>
    Inspection: {String(capabilities.inspect)} · Retry: {String(
      capabilities.retry,
    )} · Reconciliation: {String(capabilities.reconcile)}
  </p>
  {#if capabilities.inspect === true}<Button
      disabled={busy}
      onclick={() => void run("list", { limit: 50 })}>List work</Button
    >{/if}{/if}
{#each records as item}<section
    class="flex min-w-0 flex-col items-start justify-between gap-3 rounded-lg border p-3 sm:flex-row sm:items-center"
  >
    <p class="min-w-0 break-all text-sm">
      {String(item.handle)} · {String(item.category)} · {stringifyWire(
        item.state,
      )}
    </p>
    <Button
      disabled={busy}
      onclick={() => void run("read", { handle: item.handle })}
      aria-label={`Inspect ${String(item.handle)}`}>Inspect</Button
    >
  </section>{/each}
{#if selected}<pre
    class="overflow-x-auto rounded-lg bg-muted p-3 text-xs">{stringifyWire(
      selected,
    )}</pre>
  <CheckboxAdapter
    label="I confirm this recovery action"
    checked={confirmation}
    onchange={(next) => (confirmation = next)}
    disabled={busy}
  />
  {#if capabilities?.retry === true}<Button
      disabled={!confirmation || busy || pending !== null}
      onclick={() => void control("Retry")}>Retry work</Button
    >{/if}
  {#if capabilities?.reconcile === true}<label
      >Evidence reference<Input bind:value={evidence} /></label
    ><Button
      disabled={!confirmation || busy || pending !== null}
      onclick={() => void control("Reconcile")}>Reconcile work</Button
    >{/if}{/if}
{#if pending}<p>
    The recovery result is not confirmed. Preserve this request when retrying.
  </p>
  <Button disabled={busy} onclick={() => void run("control", pending!)}
    >Retry same recovery request</Button
  >{/if}
{#if result !== null}<pre>{stringifyWire(result)}</pre>{/if}{#if error}<p
    role="alert"
  >
    {error}
  </p>{/if}
