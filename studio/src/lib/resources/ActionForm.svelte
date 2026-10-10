<script lang="ts">
  import { untrack } from "svelte";
  import type {
    ResourceDescriptor,
    ActionInput,
    FieldIntent,
    WireValue,
  } from "../client/types.ts";
  import { objectInput, normalizeValue, parseWire } from "../client/codec.ts";
  import {
    captureFormDraft,
    restoreFormDraft,
    actionDraftIdentity,
  } from "./form-draft.ts";
  import type { EditorSnapshot } from "../application/editor-drafts.ts";
  import type { EditorDraft } from "../renderers/editor-draft.ts";
  import FieldHost from "../renderers/FieldHost.svelte";
  import { Textarea } from "../components/ui/textarea/index.js";
  import { Button } from "../components/ui/button/index.js";
  let {
    readonly = false,
    descriptor,
    action,
    oninvoke,
    submitDisabled = false,
    baseRevision = null,
    draftSnapshot = null,
    onDraftChange,
    restoreEpoch = 0,
  }: {
    readonly?: boolean;
    descriptor: ResourceDescriptor;
    action: ActionInput;
    oninvoke: (input: WireValue) => Promise<void>;
    submitDisabled?: boolean;
    baseRevision?: bigint | null;
    draftSnapshot?: EditorSnapshot | null;
    onDraftChange?: (
      snapshot: EditorSnapshot,
      input: WireValue | null,
    ) => Promise<void>;
    restoreEpoch?: number;
  } = $props();
  let intents = $state<Record<string, FieldIntent>>({});
  let invalid = $state<Record<string, string>>({});
  let raw = $state("null");
  let busy = $state(false);
  let error = $state("");
  const initialVersion = untrack(() => descriptor.version);
  let changed = $derived(descriptor.version !== initialVersion);
  let fields = $derived(
    action.input?.type === "object"
      ? action.input.value
      : action.input?.type === "scalar"
        ? [{ name: "input", ...action.input.value }]
        : [],
  );
  let editors = $state<Record<string, EditorDraft>>(
    untrack(() => Object.fromEntries(fields.map((field) => [field.name, {}]))),
  );
  let edited = $state(false);
  let formRevision = $state(untrack(() => baseRevision));
  let restoredEpoch = untrack(() => restoreEpoch);
  const identity = untrack(() => ({
    mode: "action" as const,
    descriptor: actionDraftIdentity(descriptor, action),
    baseRevision,
  }));
  function actionValue(): WireValue {
    if (action.input === null) return parseWire(raw, 65536, 6);
    if (action.input.type === "object") return objectInput(fields, intents);
    if (action.input.type === "unit") return null;
    const intent = intents.input;
    if (!intent || intent.mode === "omit" || intent.mode === "remove")
      throw Error("Set the action input.");
    return normalizeValue(
      action.input.value.shape,
      intent.mode === "value" ? intent.value : null,
      "input",
    );
  }
  $effect(() => {
    const ticket = restoreEpoch;
    if (ticket === restoredEpoch) return;
    restoredEpoch = ticket;
    const saved = draftSnapshot;
    if (!saved) return;
    untrack(() => {
      try {
        const restored = restoreFormDraft(saved, identity);
        intents = restored.intents;
        editors = Object.fromEntries(
          fields.map((field) => [
            field.name,
            Object.hasOwn(restored.editors, field.name)
              ? restored.editors[field.name]
              : {},
          ]),
        );
        if (action.input === null)
          raw = restored.editors.$opaque?.text ?? "null";
        formRevision = restored.baseRevision;
        invalid = Object.fromEntries(
          Object.entries(editors).map(([name, draft]) => [
            name,
            draft.invalid ?? "",
          ]),
        );
        edited = false;
      } catch {
        error = "Saved draft belongs to a different action definition.";
      }
    });
  });
  $effect(() => {
    if (!edited) return;
    const notify = untrack(() => onDraftChange);
    if (!notify) return;
    const saved = captureFormDraft({
      ...identity,
      baseRevision: formRevision,
      intents,
      editors: untrack(() => action.input === null)
        ? { $opaque: { text: raw } }
        : editors,
    });
    let value: WireValue | null = null;
    if (!Object.values(invalid).some(Boolean)) {
      try {
        value = untrack(actionValue);
      } catch {}
    }
    untrack(() => {
      void notify(saved, value).catch(() => {
        error = "Draft persistence failed. Keep this view open.";
      });
    });
  });
  async function invoke(event: SubmitEvent) {
    event.preventDefault();
    if (
      readonly ||
      submitDisabled ||
      busy ||
      changed ||
      Object.values(invalid).some(Boolean)
    )
      return;
    error = "";
    busy = true;
    try {
      const input = actionValue();
      await oninvoke(input);
    } catch (problem) {
      error = problem instanceof Error ? problem.message : "Action failed.";
    } finally {
      busy = false;
    }
  }
</script>

<form
  class="rounded-lg border bg-card p-4"
  onsubmit={invoke}
  oninput={() => (edited = true)}
>
  <h2 class="mb-2">{action.name}</h2>
  {#if changed}<p role="alert">
      Resource definition changed. Reopen the action form.
    </p>{/if}
  {#if action.input === null}<p>
      This action has opaque inputs. A typed form is unavailable.
    </p>
    <label
      >Explicit JSON input<Textarea
        aria-label={`${action.name} raw JSON input`}
        bind:value={raw}
        disabled={readonly || busy || changed}
      /></label
    >{/if}
  {#each fields as field (field.name)}<FieldHost
      descriptor={field}
      intent={intents[field.name] ?? { mode: "omit" }}
      bind:draft={editors[field.name]}
      onchange={(intent) => {
        intents = { ...intents, [field.name]: intent };
        edited = true;
      }}
      onerror={(message) => (invalid = { ...invalid, [field.name]: message })}
      readonly={readonly || busy || changed}
    />{/each}
  {#if error}<p role="alert">{error}</p>{/if}<Button
    type="submit"
    class="mt-4"
    disabled={readonly ||
      submitDisabled ||
      busy ||
      changed ||
      Object.values(invalid).some(Boolean)}>Run {action.name}</Button
  >
</form>
