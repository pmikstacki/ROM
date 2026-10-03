<script lang="ts">
  import type {
    ApplicationController,
    ApplicationState,
  } from "./controller.ts";
  import type { ResourceDescriptor } from "../client/types.ts";
  import ResourceTable from "../resources/ResourceTable.svelte";
  import ResourceForm from "../resources/ResourceForm.svelte";
  import QueryEditor from "../resources/QueryEditor.svelte";
  import ActionForm from "../resources/ActionForm.svelte";
  import ValueDisplay from "../renderers/ValueDisplay.svelte";
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
    id = $state(""),
    deleteConfirm = $state(false);
  const blocked = $derived(
    snapshot.busy ||
      snapshot.pending?.state === "unknown" ||
      snapshot.pending?.state === "pending",
  );
</script>

<h2>{descriptor.kind}</h2>
<QueryEditor
  {descriptor}
  onchange={(query) => void controller.selectKind(descriptor.kind, query)}
/>
<div class="toolbar">
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
{#if create}<section aria-label="Create Resource">
    <label>Resource ID<Input bind:value={id} /></label><ResourceForm
      {descriptor}
      mode="create"
      submit={async (input) => {
        await controller.mutate(id, null, input);
        create = false;
      }}
    />
  </section>{/if}
<ResourceTable
  {descriptor}
  rows={snapshot.rows}
  onselect={(row) => void controller.selectRow(row.key.id)}
/>
{#if snapshot.selected && snapshot.selected.key.kind === descriptor.kind}
  {#key snapshot.selected.key.id}
    <section aria-label="Resource details">
      <h2>Resource {snapshot.selected.key.id}</h2>
      <p>Revision {String(snapshot.selected.revision)}</p>
      {#if snapshot.selected.value === null}<p>
          This Resource is deleted.
        </p>{:else}
        <dl>
          {#each descriptor.fields as field}<dt>{field.name}</dt>
            <dd>
              <ValueDisplay
                descriptor={field}
                value={snapshot.selected.value[field.name]}
              />
            </dd>{/each}
        </dl>
        <fieldset disabled={blocked}>
          <ResourceForm
            {descriptor}
            value={snapshot.selected.value}
            mode="patch"
            submit={async (input) => {
              await controller.mutate(
                snapshot.selected!.key.id,
                snapshot.selected!.revision,
                input,
              );
            }}
          />
          {#each descriptor.action_inputs as action (action.name)}<ActionForm
              {descriptor}
              {action}
              oninvoke={async (input) => {
                await controller.mutate(
                  snapshot.selected!.key.id,
                  snapshot.selected!.revision,
                  { type: "action", input: { name: action.name, input } },
                );
              }}
            />{/each}
          <label
            ><input type="checkbox" bind:checked={deleteConfirm} />Confirm
            deletion of {snapshot.selected.key.id}</label
          ><Button
            variant="destructive"
            disabled={!deleteConfirm}
            onclick={() =>
              void controller
                .mutate(
                  snapshot.selected!.key.id,
                  snapshot.selected!.revision,
                  {
                    type: "delete",
                  },
                )
                .catch(() => {})}>Delete Resource</Button
          >
        </fieldset>
      {/if}
    </section>
  {/key}
{/if}
