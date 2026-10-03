<script lang="ts">
  import type { FieldDescriptor, FieldIntent } from "../client/types.ts";
  import ValueEditor from "./ValueEditor.svelte";
  import { findRenderer } from "./registry.ts";
  import { defaultValue } from "./default-value.ts";
  let {
    descriptor,
    intent,
    onchange,
    readonly = false,
    error = "",
    onerror = () => {},
  }: {
    descriptor: FieldDescriptor;
    intent: FieldIntent;
    onchange: (intent: FieldIntent) => void;
    readonly?: boolean;
    error?: string;
    onerror?: (message: string) => void;
  } = $props();
  let optional = $derived(descriptor.shape.type === "optional");
  let nullable = $derived(
    descriptor.shape.type === "nullable" ||
      (descriptor.shape.type === "optional" &&
        descriptor.shape.value.type === "nullable"),
  );
  function mode(next: string) {
    onerror("");
    onchange(
      next === "value"
        ? { mode: "value", value: defaultValue(descriptor.shape) }
        : { mode: next as "omit" | "null" | "remove" },
    );
  }
</script>

<fieldset class="resource-field">
  {#if descriptor.codec && !findRenderer(descriptor.codec)}<p>
      Custom editor unavailable for {descriptor.codec.name} version {descriptor
        .codec.version}. Existing values are preserved.
    </p>{/if}
  <legend>{descriptor.name}</legend>
  <label
    >Operation<select
      aria-label={`${descriptor.name} mode`}
      value={intent.mode}
      disabled={readonly}
      onchange={(event) => mode(event.currentTarget.value)}
      ><option value="omit">Unchanged / omitted</option><option value="value"
        >Set value</option
      >{#if nullable}<option value="null">Set null</option
        >{/if}{#if optional}<option value="remove">Remove value</option
        >{/if}</select
    ></label
  >
  {#if intent.mode === "value"}<ValueEditor
      shape={descriptor.shape}
      codec={descriptor.codec}
      value={intent.value}
      onchange={(value) => onchange({ mode: "value", value })}
      label={descriptor.name}
      {readonly}
      {onerror}
    />{/if}
  {#if error}<p role="alert">{error}</p>{/if}
</fieldset>
