<script lang="ts">
  import { untrack } from "svelte";
  import type {
    ResourceDescriptor,
    FieldIntent,
    WireObject,
  } from "../client/types.ts";
  import { objectInput, patchInput } from "../client/codec.ts";
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
  }: {
    readonly?: boolean;
    descriptor: ResourceDescriptor;
    value?: WireObject;
    mode?: "create" | "replace" | "patch";
    direct?: boolean;
    submit: (input: ResourceFormSubmission) => Promise<void>;
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
    summary = "";
    busy = true;
    try {
      const input =
        mode === "patch"
          ? {
              type: "patch" as const,
              input: patchInput(descriptor.fields, intents),
            }
          : { type: mode, input: objectInput(descriptor.fields, intents) };
      await submit(input);
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
  }
</script>

<form class="space-y-4" onsubmit={apply}>
  {#if changed}<p role="alert">
      Resource definition changed. The draft is preserved. Reopen the form
      before submitting.
    </p>{/if}
  {#each primaryFields as field (field.name)}<FieldHost
      descriptor={field}
      direct={direct && mode === "patch"}
      current={value[field.name]}
      intent={intents[field.name] ?? { mode: "omit" }}
      onchange={(intent) => update(field.name, intent)}
      onerror={(error) => (invalid = { ...invalid, [field.name]: error })}
      readonly={readonly || busy || changed}
    />{/each}
  {#if advancedFields.length}<details
      class="rounded-lg border border-dashed p-3"
    >
      <summary class="cursor-pointer text-sm font-medium">
        Advanced fields · {advancedFields.length} read-only
      </summary>
      <div class="mt-3 space-y-3">
        {#each advancedFields as field (field.name)}<FieldHost
            descriptor={field}
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
    disabled={readonly ||
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
