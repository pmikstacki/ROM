<script lang="ts">
  import { untrack } from "svelte";
  import type {
    ResourceDescriptor,
    ProjectedView,
    Operation,
  } from "../client/types.ts";
  import ResourceForm from "../resources/ResourceForm.svelte";
  import ActionForm from "../resources/ActionForm.svelte";
  import ValueDisplay from "../renderers/ValueDisplay.svelte";
  import CheckboxAdapter from "../renderers/CheckboxAdapter.svelte";
  import { Button } from "../components/ui/button/index.js";
  let {
    descriptor,
    selected,
    blocked,
    onmutate,
    onreload,
  }: {
    descriptor: ResourceDescriptor;
    selected: ProjectedView;
    blocked: boolean;
    onmutate: (expected: bigint, operation: Operation) => Promise<void>;
    onreload: () => void;
  } = $props();
  const draftRevision = untrack(() => selected.revision);
  const stale = $derived(selected.revision !== draftRevision);
  let deleteConfirm = $state(false);
</script>

<section class="space-y-5" aria-label="Resource details">
  <h2 class="text-lg font-semibold tracking-tight">
    Resource {selected.key.id}
  </h2>
  <p class="text-xs text-muted-foreground">
    Revision {String(selected.revision)}
  </p>
  {#if stale}<p role="status">
      This Resource changed after the draft was opened.
    </p>
    <p>Reload to discard the draft and edit the current revision.</p>
    <Button disabled={blocked} onclick={onreload}
      >Discard draft and reload</Button
    >{/if}
  {#if selected.value === null}<p>This Resource is deleted.</p>{:else}
    <dl
      class="grid grid-cols-[minmax(0,1fr)_minmax(0,2fr)] gap-x-4 gap-y-2 rounded-lg bg-muted/40 p-3 text-sm"
    >
      {#each descriptor.fields as field}<dt>{field.name}</dt>
        <dd>
          <ValueDisplay descriptor={field} value={selected.value[field.name]} />
        </dd>{/each}
    </dl>
    <fieldset class="space-y-4" disabled={blocked || stale}>
      <ResourceForm
        {descriptor}
        readonly={blocked || stale}
        value={selected.value}
        mode="patch"
        submit={async (input) => {
          await onmutate(draftRevision, input);
        }}
      />
      {#each descriptor.action_inputs as action (action.name)}<ActionForm
          {descriptor}
          readonly={blocked || stale}
          {action}
          oninvoke={async (input) => {
            await onmutate(draftRevision, {
              type: "action",
              input: { name: action.name, input },
            });
          }}
        />{/each}
      <CheckboxAdapter
        label={`Confirm deletion of ${selected.key.id}`}
        checked={deleteConfirm}
        onchange={(next) => (deleteConfirm = next)}
        disabled={blocked || stale}
      /><Button
        variant="destructive"
        disabled={!deleteConfirm}
        onclick={() =>
          void onmutate(draftRevision, { type: "delete" }).catch(() => {})}
        >Delete Resource</Button
      >
    </fieldset>
  {/if}
</section>
