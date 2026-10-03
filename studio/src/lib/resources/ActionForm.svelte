<script lang="ts">
  import { untrack } from "svelte";
  import type {
    ResourceDescriptor,
    ActionInput,
    FieldIntent,
    WireValue,
  } from "../client/types.ts";
  import { objectInput, normalizeValue, parseWire } from "../client/codec.ts";
  import FieldHost from "../renderers/FieldHost.svelte";
  import { Button } from "../components/ui/button/index.js";
  let {
    descriptor,
    action,
    oninvoke,
  }: {
    descriptor: ResourceDescriptor;
    action: ActionInput;
    oninvoke: (input: WireValue) => Promise<void>;
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
  async function invoke(event: SubmitEvent) {
    event.preventDefault();
    error = "";
    busy = true;
    try {
      let input: WireValue = null;
      if (action.input === null) input = parseWire(raw, 65536, 6);
      else if (action.input.type === "object")
        input = objectInput(fields, intents);
      else if (action.input.type === "scalar") {
        let intent = intents.input;
        if (!intent || intent.mode === "omit" || intent.mode === "remove")
          throw Error("Set the action input.");
        input = normalizeValue(
          action.input.value.shape,
          intent.mode === "value" ? intent.value : null,
          "input",
        );
      }
      await oninvoke(input);
    } catch (problem) {
      error = problem instanceof Error ? problem.message : "Action failed.";
    } finally {
      busy = false;
    }
  }
</script>

<form onsubmit={invoke}>
  <h2>{action.name}</h2>
  {#if changed}<p role="alert">
      Resource definition changed. Reopen the action form.
    </p>{/if}
  {#if action.input === null}<p>
      This action has opaque inputs. A typed form is unavailable.
    </p>
    <label
      >Explicit JSON input<textarea
        aria-label={`${action.name} raw JSON input`}
        bind:value={raw}
        disabled={busy || changed}></textarea></label
    >{/if}
  {#each fields as field (field.name)}<FieldHost
      descriptor={field}
      intent={intents[field.name] ?? { mode: "omit" }}
      onchange={(intent) => (intents = { ...intents, [field.name]: intent })}
      onerror={(message) => (invalid = { ...invalid, [field.name]: message })}
      readonly={busy || changed}
    />{/each}
  {#if error}<p role="alert">{error}</p>{/if}<Button
    type="submit"
    disabled={busy || changed || Object.values(invalid).some(Boolean)}
    >Run {action.name}</Button
  >
</form>
