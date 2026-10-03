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

<section aria-label="Resource details">
  <h2>Resource {selected.key.id}</h2>
  <p>Revision {String(selected.revision)}</p>
  {#if stale}<p role="status">
      This Resource changed after the draft was opened.
    </p>
    <p>Reload to discard the draft and edit the current revision.</p>
    <Button disabled={blocked} onclick={onreload}
      >Discard draft and reload</Button
    >{/if}
  {#if selected.value === null}<p>This Resource is deleted.</p>{:else}
    <dl>
      {#each descriptor.fields as field}<dt>{field.name}</dt>
        <dd>
          <ValueDisplay descriptor={field} value={selected.value[field.name]} />
        </dd>{/each}
    </dl>
    <fieldset disabled={blocked || stale}>
      <ResourceForm
        {descriptor}
        value={selected.value}
        mode="patch"
        submit={async (input) => {
          await onmutate(draftRevision, input);
        }}
      />
      {#each descriptor.action_inputs as action (action.name)}<ActionForm
          {descriptor}
          {action}
          oninvoke={async (input) => {
            await onmutate(draftRevision, {
              type: "action",
              input: { name: action.name, input },
            });
          }}
        />{/each}
      <label
        ><input type="checkbox" bind:checked={deleteConfirm} />Confirm deletion
        of {selected.key.id}</label
      ><Button
        variant="destructive"
        disabled={!deleteConfirm}
        onclick={() =>
          void onmutate(draftRevision, { type: "delete" }).catch(() => {})}
        >Delete Resource</Button
      >
    </fieldset>
  {/if}
</section>
