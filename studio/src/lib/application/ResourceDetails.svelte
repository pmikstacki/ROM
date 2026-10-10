<script lang="ts">
  import { untrack } from "svelte";
  import type {
    ResourceDescriptor,
    ProjectedView,
    Operation,
  } from "../client/types.ts";
  import ResourceForm from "../resources/ResourceForm.svelte";
  import ActionForm from "../resources/ActionForm.svelte";
  import CheckboxAdapter from "../renderers/CheckboxAdapter.svelte";
  import { Button } from "../components/ui/button/index.js";
  import ResourceSummary from "../presentation/ResourceSummary.svelte";
  import type { EditorSnapshot } from "./editor-drafts.ts";
  import {
    formFrameDefinition,
    mergeFormFrames,
    readFormFrames,
  } from "../resources/form-draft.ts";
  import type { ResourceFormSubmission } from "../resources/form-types.ts";
  let {
    descriptor,
    selected,
    blocked,
    onmutate,
    onreload,
    managed = false,
    mutationAllowed = true,
    draftSnapshot = null,
    draftWriter,
    restoreEpoch = 0,
  }: {
    descriptor: ResourceDescriptor;
    selected: ProjectedView;
    blocked: boolean;
    onmutate: (expected: bigint, operation: Operation) => Promise<void>;
    onreload: () => void;
    managed?: boolean;
    mutationAllowed?: boolean;
    draftSnapshot?: EditorSnapshot | null;
    draftWriter?: {
      stage: (
        snapshot:
          EditorSnapshot | ((current: EditorSnapshot | null) => EditorSnapshot),
        operation: Operation | null,
      ) => Promise<void>;
      refuse: () => void;
    };
    restoreEpoch?: number;
  } = $props();
  const frameDefinition = untrack(() => formFrameDefinition(descriptor));
  const frames = $derived.by(() => {
    if (!draftSnapshot)
      return { resource: null, actions: {} as Record<string, EditorSnapshot> };
    try {
      return readFormFrames(draftSnapshot, frameDefinition);
    } catch {
      return { resource: null, actions: {} as Record<string, EditorSnapshot> };
    }
  });
  const initialRevision = untrack(() => selected.revision);
  const draftRevision = $derived(
    draftSnapshot?.baseRevision ?? initialRevision,
  );
  const stale = $derived(selected.revision !== draftRevision);
  let deleteConfirm = $state(false);
</script>

<section class="space-y-5" aria-label="Resource details">
  <ResourceSummary {descriptor} resource={selected} />
  {#if stale}<p role="status">
      This Resource changed after the draft was opened.
    </p>
    <p>Reload to discard the draft and edit the current revision.</p>
    <Button disabled={blocked} onclick={onreload}
      >Discard draft and reload</Button
    >{/if}
  {#if selected.value === null}<p>This Resource is deleted.</p>{:else}
    <fieldset class="space-y-4">
      <ResourceForm
        {descriptor}
        direct
        readonly={!managed && (blocked || stale)}
        submitDisabled={blocked || stale || !mutationAllowed}
        baseRevision={draftRevision}
        draftSnapshot={frames.resource}
        {restoreEpoch}
        onDraftChange={draftWriter
          ? (next, input: ResourceFormSubmission | null) =>
              draftWriter!.stage(
                (current) =>
                  mergeFormFrames(
                    current,
                    { type: "resource" },
                    next,
                    frameDefinition,
                  ),
                input,
              )
          : undefined}
        value={selected.value}
        mode="patch"
        submit={async (input) => {
          await onmutate(draftRevision, input);
        }}
      />
      {#each descriptor.action_inputs as action (action.name)}<ActionForm
          {descriptor}
          readonly={!managed && (blocked || stale)}
          submitDisabled={blocked || stale || !mutationAllowed}
          baseRevision={draftRevision}
          draftSnapshot={Object.hasOwn(frames.actions, action.name)
            ? frames.actions[action.name]
            : null}
          {restoreEpoch}
          onDraftChange={draftWriter
            ? (next, input) =>
                draftWriter!.stage(
                  (current) =>
                    mergeFormFrames(
                      current,
                      { type: "action", name: action.name },
                      next,
                      frameDefinition,
                    ),
                  input === null
                    ? null
                    : { type: "action", input: { name: action.name, input } },
                )
            : undefined}
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
        disabled={blocked || stale || !deleteConfirm}
        onclick={() =>
          void onmutate(draftRevision, { type: "delete" }).catch(() => {})}
        >Delete Resource</Button
      >
    </fieldset>
  {/if}
</section>
