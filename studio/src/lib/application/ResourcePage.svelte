<script lang="ts">
  import type {
    ApplicationController,
    ApplicationState,
  } from "./controller.ts";
  import type { ResourceDescriptor } from "../client/types.ts";
  import ResourceTable from "../resources/ResourceTable.svelte";
  import ResourceForm from "../resources/ResourceForm.svelte";
  import QueryEditor from "../resources/QueryEditor.svelte";
  import ResourceDetails from "./ResourceDetails.svelte";

  import { Button } from "../components/ui/button/index.js";
  import { Input } from "../components/ui/input/index.js";
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
  const blocked = $derived(
    snapshot.busy ||
      snapshot.pending?.state === "unknown" ||
      snapshot.pending?.state === "pending",
  );
</script>

<h1 class="text-xl font-semibold tracking-tight">{descriptor.kind}</h1>
<QueryEditor
  {descriptor}
  onchange={(query) => void controller.selectKind(descriptor.kind, query)}
/>
<div class="toolbar flex flex-wrap gap-2">
  <Button disabled={snapshot.busy} onclick={() => void controller.refresh()}
    >Refresh</Button
  ><Button
    disabled={snapshot.busy}
    onclick={() =>
      snapshot.live ? controller.stopLive() : void controller.observe()}
    >{snapshot.live ? "Stop live query" : "Observe live query"}</Button
  ><Button disabled={blocked} onclick={() => (create = !create)}
    >Create Resource</Button
  >
</div>
{#if snapshot.busy}<p role="status">Loading…</p>{/if}
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
<nav
  class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground"
  aria-label="Query pages"
>
  <Button
    disabled={snapshot.busy || snapshot.page === 1}
    onclick={() => void controller.firstPage()}>First page</Button
  ><Button
    disabled={snapshot.busy || !snapshot.hasPrevious}
    onclick={() => void controller.previousPage()}>Previous page</Button
  ><span role="status">Moving page {snapshot.page}</span><Button
    disabled={snapshot.busy ||
      snapshot.rows.length < (snapshot.query.limit ?? 50)}
    onclick={() => void controller.nextPage()}>Next page</Button
  >
</nav>
<div
  class={snapshot.selected?.key.kind === descriptor.kind
    ? "grid items-start gap-4 xl:grid-cols-[minmax(0,1fr)_24rem]"
    : "grid gap-4"}
>
  <div class="min-w-0">
    <ResourceTable
      {descriptor}
      rows={snapshot.rows}
      onselect={(row) => {
        void controller.selectRow(row.key.id);
      }}
    />
  </div>
  {#snippet details()}
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
          onreload={() => void controller.selectRow(snapshot.selected!.key.id)}
        />{/key}
    {/if}
  {/snippet}
  {#if snapshot.selected && snapshot.selected.key.kind === descriptor.kind}
    <aside class="rounded-lg border bg-card p-5 xl:sticky xl:top-4">
      {@render details()}
    </aside>
  {/if}
</div>
