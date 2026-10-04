<script lang="ts">
  import type { FieldDescriptor, FieldIntent } from "../client/types.ts";
  import SelectAdapter from "./SelectAdapter.svelte";
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
  let unknownCodec = $derived(
    !!descriptor.codec && !findRenderer(descriptor.codec),
  );
  let optional = $derived(descriptor.shape.type === "optional");
  let nullable = $derived(
    descriptor.shape.type === "nullable" ||
      (descriptor.shape.type === "optional" &&
        descriptor.shape.value.type === "nullable"),
  );
  function mode(next: string) {
    if (readonly || unknownCodec) return;
    onerror("");
    onchange(
      next === "value"
        ? { mode: "value", value: defaultValue(descriptor.shape) }
        : { mode: next as "omit" | "null" | "remove" },
    );
  }
</script>

<fieldset class="resource-field space-y-2 rounded-lg border p-3">
  {#if descriptor.codec && !findRenderer(descriptor.codec)}<p>
      Custom editor unavailable for {descriptor.codec.name} version {descriptor
        .codec.version}. Existing values are preserved.
    </p>{/if}
  <legend class="px-1 text-sm font-medium">{descriptor.name}</legend>
  <div class="space-y-1.5">
    <span class="text-xs text-muted-foreground">Operation</span>
    <SelectAdapter
      label={`${descriptor.name} mode`}
      value={intent.mode}
      disabled={readonly || unknownCodec}
      onchange={mode}
      options={[
        { value: "omit", label: "Unchanged / omitted" },
        { value: "value", label: "Set value" },
        ...(nullable ? [{ value: "null", label: "Set null" }] : []),
        ...(optional ? [{ value: "remove", label: "Remove value" }] : []),
      ]}
    />
  </div>
  {#if intent.mode === "value"}<ValueEditor
      shape={descriptor.shape}
      codec={descriptor.codec}
      codecWrappers={descriptor.codec_wrappers}
      value={intent.value}
      onchange={(value) => onchange({ mode: "value", value })}
      label={descriptor.name}
      {readonly}
      {onerror}
    />{/if}
  {#if error}<p role="alert">{error}</p>{/if}
</fieldset>
