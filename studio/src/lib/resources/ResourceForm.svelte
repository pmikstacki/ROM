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
  import { Button } from "../components/ui/button/index.js";
  let {
    readonly = false,
    descriptor,
    value = {},
    mode = "patch",
    submit,
  }: {
    readonly?: boolean;
    descriptor: ResourceDescriptor;
    value?: WireObject;
    mode?: "create" | "replace" | "patch";
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
  {#each descriptor.fields as field (field.name)}<FieldHost
      descriptor={field}
      intent={intents[field.name] ?? { mode: "omit" }}
      onchange={(intent) => update(field.name, intent)}
      onerror={(error) => (invalid = { ...invalid, [field.name]: error })}
      readonly={readonly || busy || changed}
    />{/each}
  {#if summary}<p role="alert">{summary}</p>{/if}
  <Button
    type="submit"
    disabled={readonly ||
      busy ||
      changed ||
      Object.values(invalid).some(Boolean)}
    >{busy
      ? "Submitting…"
      : mode === "patch"
        ? "Apply patch"
        : mode === "create"
          ? "Create resource"
          : "Replace resource"}</Button
  >
</form>
