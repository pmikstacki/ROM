<script lang="ts">
  import { untrack } from "svelte";
  import type {
    ApplicationController,
    ApplicationState,
  } from "./controller.ts";
  import type { ResourceDescriptor } from "../client/types.ts";
  import type { EditorSnapshot } from "./editor-drafts.ts";
  import { resourceDraftIdentity } from "../resources/form-draft.ts";
  import ResourceForm from "../resources/ResourceForm.svelte";
  import { Input } from "../components/ui/input/index.js";
  import { Button } from "../components/ui/button/index.js";
  import * as AlertDialog from "../components/ui/alert-dialog/index.js";
  let {
    controller,
    snapshot,
    descriptor,
    oncreated,
  }: {
    controller: ApplicationController;
    snapshot: ApplicationState;
    descriptor: ResourceDescriptor;
    oncreated: () => void;
  } = $props();
  const managed = untrack(() => controller.creationSupported);
  const writer = untrack(() =>
    managed ? controller.createCreationWriter() : null,
  );
  function initialDraft(current: ResourceDescriptor) {
    return {
      mode: "resource" as const,
      descriptor: resourceDraftIdentity(current, "create"),
      baseRevision: null,
      intents: Object.fromEntries(
        current.fields.map((field) => [field.name, { mode: "omit" }]),
      ),
      editors: {},
    };
  }
  let formDescriptor = $state.raw(untrack(() => descriptor));
  let form = $state.raw<EditorSnapshot>(
      untrack(() => initialDraft(descriptor)),
    ),
    id = $state(""),
    restoreEpoch = $state(0),
    formGeneration = $state(0),
    error = $state(""),
    confirming = $state(false);
  const recovery = $derived(snapshot.creation?.recovery);
  const pending = $derived(
    !!recovery?.state.hasUnresolvedIntent ||
      recovery?.state.phase === "storage_error",
  );
  const busy = $derived(
    !!snapshot.creation?.busy || snapshot.creation?.editor.status === "writing",
  );
  const allowed = $derived(snapshot.session?.mutationAllowed ?? !managed);
  const changedDefinition = $derived(
    resourceDraftIdentity(formDescriptor, "create") !==
      resourceDraftIdentity(descriptor, "create"),
  );
  async function stage(next: EditorSnapshot = form) {
    form = next;
    if (!writer) return;
    try {
      await writer.stage({ id, form: next });
      error = "";
    } catch (problem) {
      error =
        problem instanceof Error
          ? problem.message
          : "Draft persistence failed. Keep this form open.";
      throw problem;
    }
  }
  async function restore() {
    try {
      await controller.restoreCreation();
      const saved = controller.state.creation?.editor.snapshot;
      if (saved) {
        id = saved.id;
        form = saved.form;
        restoreEpoch++;
      }
      error = "";
    } catch (problem) {
      error =
        problem instanceof Error
          ? problem.message
          : "Creation restoration failed.";
    }
  }
  async function retry() {
    try {
      await controller.retryCreation();
      error = "";
      if (!controller.state.creation?.editor.snapshot) oncreated();
    } catch (problem) {
      error =
        problem instanceof Error ? problem.message : "Creation retry failed.";
    }
  }
  async function discard() {
    try {
      await controller.discardCreation(true);
      id = "";
      formDescriptor = descriptor;
      form = initialDraft(descriptor);
      restoreEpoch = 0;
      formGeneration++;
      error = "";
      confirming = false;
    } catch (problem) {
      error =
        problem instanceof Error
          ? problem.message
          : "Local creation discard failed.";
    }
  }
</script>

<section
  class="space-y-3 rounded-lg border bg-card p-4"
  aria-label="Create Resource"
>
  <div class="flex flex-wrap items-center justify-between gap-2">
    <h2 class="text-sm font-semibold">Create Resource</h2>
    {#if managed}<div class="flex flex-wrap gap-2">
        <Button
          variant="outline"
          size="sm"
          disabled={busy || pending || !allowed}
          onclick={() => void restore()}>Restore creation</Button
        >
        <Button
          variant="ghost"
          size="sm"
          disabled={busy || !allowed}
          onclick={() => (confirming = true)}>Discard local creation</Button
        >
      </div>{/if}
  </div>
  <label class="flex min-w-0 items-center gap-3 text-sm"
    ><span class="shrink-0">Resource ID</span><Input
      class="min-w-0"
      bind:value={id}
      disabled={changedDefinition}
      oninput={() => void stage().catch(() => {})}
    /></label
  >
  {#if managed}<p class="text-xs text-muted-foreground" role="status">
      {snapshot.creation?.editor.status === "writing"
        ? "Saving local draft…"
        : snapshot.creation?.editor.status === "saved"
          ? "Local draft saved"
          : "Drafts restore only when requested."}
    </p>{/if}
  {#if recovery}<div class="space-y-2" role="status">
      <p class="text-sm">
        Creation {recovery.state.phase}. Commit knowledge: {recovery.state
          .commitKnowledge}.
      </p>
      {#if pending}<p class="text-xs text-muted-foreground">
          This creation may already exist. Retry the original command to recover
          its result.
        </p>
        <Button
          size="sm"
          disabled={busy ||
            !allowed ||
            recovery.state.phase === "storage_error"}
          onclick={() => void retry()}>Retry creation</Button
        >
      {/if}
    </div>{/if}
  {#if error}<p role="alert" class="text-sm text-destructive">{error}</p>{/if}
  {#if changedDefinition}<p role="status" class="text-sm text-muted-foreground">
      Resource definition changed. Retry the original creation or discard local
      edits before using the new definition.
    </p>{/if}
  {#key formGeneration}<ResourceForm
      descriptor={formDescriptor}
      readonly={changedDefinition}
      mode="create"
      draftSnapshot={form}
      {restoreEpoch}
      submitDisabled={busy ||
        changedDefinition ||
        pending ||
        !allowed ||
        snapshot.creation?.editor.status === "error"}
      onDraftChange={writer ? (next) => stage(next) : undefined}
      submit={async (input) => {
        await controller.mutate(id, null, input);
        if (!managed || !controller.state.creation?.editor.snapshot)
          oncreated();
      }}
    />{/key}
  <AlertDialog.Root bind:open={confirming}
    ><AlertDialog.Content>
      <AlertDialog.Header
        ><AlertDialog.Title>Discard local creation?</AlertDialog.Title
        ><AlertDialog.Description
          >The creation may already exist. This removes only the local draft and
          recovery record. It does not undo a backend commit.</AlertDialog.Description
        ></AlertDialog.Header
      >
      <AlertDialog.Footer
        ><AlertDialog.Cancel>Keep creation</AlertDialog.Cancel
        ><AlertDialog.Action onclick={() => void discard()}
          >Discard local records</AlertDialog.Action
        ></AlertDialog.Footer
      >
    </AlertDialog.Content></AlertDialog.Root
  >
</section>
