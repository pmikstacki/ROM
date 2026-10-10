<script lang="ts">
  import { untrack } from "svelte";
  import type {
    ResourceDescriptor,
    FieldIntent,
    WireObject,
    WireValue,
  } from "../client/types.ts";
  import { objectInput, patchInput, stringifyWire } from "../client/codec.ts";
  import {
    captureFormDraft,
    restoreFormDraft,
    resourceDraftIdentity,
  } from "./form-draft.ts";
  import type { EditorSnapshot } from "../application/editor-drafts.ts";
  import type { EditorDraft } from "../renderers/editor-draft.ts";
  import type { ResourceFormSubmission } from "./form-types.ts";
  import FieldHost from "../renderers/FieldHost.svelte";
  import { findRenderer } from "../renderers/registry.ts";
  import { Button } from "../components/ui/button/index.js";
  let {
    readonly = false,
    descriptor,
    value = {},
    mode = "patch",
    direct = false,
    submit,
    submitDisabled = false,
    baseRevision = null,
    draftSnapshot = null,
    onDraftChange,
    restoreEpoch = 0,
  }: {
    readonly?: boolean;
    descriptor: ResourceDescriptor;
    value?: WireObject;
    mode?: "create" | "replace" | "patch";
    direct?: boolean;
    submit: (input: ResourceFormSubmission) => Promise<void>;
    submitDisabled?: boolean;
    baseRevision?: bigint | null;
    draftSnapshot?: EditorSnapshot | null;
    onDraftChange?: (
      snapshot: EditorSnapshot,
      input: ResourceFormSubmission | null,
    ) => Promise<void>;
    restoreEpoch?: number;
  } = $props();
  let intents = $state<Record<string, FieldIntent>>(
    untrack(() =>
      Object.fromEntries(
        descriptor.fields.map((f) => [
          f.name,
          mode === "patch"
            ? { mode: "omit" }
            : Object.hasOwn(value, f.name)
              ? value[f.name] === null
                ? { mode: "null" }
                : { mode: "value", value: value[f.name] }
              : { mode: "omit" },
        ]),
      ),
    ),
  );
  let invalid = $state<Record<string, string>>({});
  let summary = $state("");
  let busy = $state(false);
  let editors = $state<Record<string, EditorDraft>>(
    untrack(() =>
      Object.fromEntries(descriptor.fields.map((field) => [field.name, {}])),
    ),
  );
  let edited = $state(false);
  let formRevision = $state(untrack(() => baseRevision));
  let restoredEpoch = untrack(() => restoreEpoch);
  const formIdentity = untrack(() => ({
    mode: "resource" as const,
    descriptor: resourceDraftIdentity(descriptor, mode),
    baseRevision,
  }));
  function input(): ResourceFormSubmission {
    return mode === "patch"
      ? { type: "patch", input: patchInput(descriptor.fields, intents) }
      : { type: mode, input: objectInput(descriptor.fields, intents) };
  }
  $effect(() => {
    const ticket = restoreEpoch;
    if (ticket === restoredEpoch) return;
    restoredEpoch = ticket;
    const saved = draftSnapshot;
    if (!saved || saved.mode !== "resource") return;
    untrack(() => {
      try {
        const local = captureFormDraft({
          ...formIdentity,
          baseRevision: formRevision,
          intents,
          editors,
        });
        if (
          stringifyWire(local as unknown as WireValue) ===
          stringifyWire(saved as unknown as WireValue)
        )
          return;
        const restored = restoreFormDraft(saved, formIdentity);
        intents = restored.intents;
        editors = Object.fromEntries(
          descriptor.fields.map((field) => [
            field.name,
            Object.hasOwn(restored.editors, field.name)
              ? restored.editors[field.name]
              : {},
          ]),
        );
        formRevision = restored.baseRevision;
        invalid = Object.fromEntries(
          Object.entries(editors).map(([name, draft]) => [
            name,
            draft.invalid ?? "",
          ]),
        );
        edited = false;
      } catch {
        summary = "Saved draft belongs to a different Resource definition.";
      }
    });
  });
  $effect(() => {
    if (!edited) return;
    const notify = untrack(() => onDraftChange);
    if (!notify) return;
    const saved = captureFormDraft({
      ...formIdentity,
      baseRevision: formRevision,
      intents,
      editors,
    });
    const valid = !Object.values(invalid).some(Boolean);
    const changedInput = Object.values(intents).some(
      (intent) => intent.mode !== "omit",
    );
    let operation: ResourceFormSubmission | null = null;
    if (valid && (mode !== "patch" || changedInput)) {
      try {
        operation = untrack(input);
      } catch {}
    }
    untrack(() => {
      void notify(saved, operation).catch(() => {
        summary = "Draft persistence failed. Keep this view open.";
      });
    });
  });
  let changedCount = $derived(
    Object.values(intents).filter((intent) => intent.mode !== "omit").length,
  );
  let advancedFields = $derived(
    direct && mode === "patch"
      ? descriptor.fields.filter(
          (field) => field.codec && !findRenderer(field.codec),
        )
      : [],
  );
  let primaryFields = $derived(
    direct && mode === "patch"
      ? descriptor.fields.filter(
          (field) => !field.codec || findRenderer(field.codec),
        )
      : descriptor.fields,
  );
  const initialVersion = untrack(() => descriptor.version);
  let changed = $derived(descriptor.version !== initialVersion);
  async function apply(event: SubmitEvent) {
    event.preventDefault();
    if (
      readonly ||
      submitDisabled ||
      busy ||
      changed ||
      Object.values(invalid).some(Boolean)
    )
      return;
    summary = "";
    busy = true;
    try {
      await submit(input());
    } catch (error) {
      summary = error instanceof Error ? error.message : "Operation failed.";
    } finally {
      busy = false;
    }
  }
  function update(name: string, intent: FieldIntent) {
    const previous = intents[name];
    if (
      !direct &&
      previous?.mode === "omit" &&
      intent.mode === "value" &&
      Object.hasOwn(value, name) &&
      value[name] !== null
    )
      intent = { mode: "value", value: value[name] };
    intents = { ...intents, [name]: intent };
    edited = true;
  }
</script>

<form onsubmit={apply} oninput={() => (edited = true)}>
  {#if changed}<p role="alert">
      Resource definition changed. The draft is preserved. Reopen the form
      before submitting.
    </p>{/if}
  {#each primaryFields as field (field.name)}<FieldHost
      descriptor={field}
      displayLabel={descriptor.presentation?.fields?.[field.name]?.label}
      help={descriptor.presentation?.fields?.[field.name]?.help}
      direct={direct && mode === "patch"}
      current={value[field.name]}
      intent={intents[field.name] ?? { mode: "omit" }}
      bind:draft={editors[field.name]}
      onchange={(intent) => update(field.name, intent)}
      onerror={(error) => (invalid = { ...invalid, [field.name]: error })}
      readonly={readonly || busy || changed}
    />{/each}
  {#if advancedFields.length}<details class="border-b border-border/60 py-3">
      <summary class="cursor-pointer text-sm font-medium">
        Advanced fields · {advancedFields.length} read-only
      </summary>
      <div class="mt-3 space-y-3">
        {#each advancedFields as field (field.name)}<FieldHost
            descriptor={field}
            displayLabel={descriptor.presentation?.fields?.[field.name]?.label}
            help={descriptor.presentation?.fields?.[field.name]?.help}
            direct
            current={value[field.name]}
            intent={{ mode: "omit" }}
            onchange={() => {}}
            readonly
          />{/each}
      </div>
    </details>{/if}
  {#if summary}<p role="alert">{summary}</p>{/if}
  <Button
    type="submit"
    class="mt-4"
    disabled={readonly ||
      submitDisabled ||
      busy ||
      changed ||
      (direct && mode === "patch" && changedCount === 0) ||
      Object.values(invalid).some(Boolean)}
    >{busy
      ? "Submitting…"
      : mode === "patch"
        ? direct
          ? changedCount === 0
            ? "Save changes"
            : `Save ${changedCount} ${changedCount === 1 ? "change" : "changes"}`
          : "Apply patch"
        : mode === "create"
          ? "Create resource"
          : "Replace resource"}</Button
  >
</form>
