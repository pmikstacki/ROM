<script lang="ts">
  import type {
    FieldDescriptor,
    FieldIntent,
    WireValue,
  } from "../client/types.ts";
  import SelectAdapter from "./SelectAdapter.svelte";
  import ValueDisplay from "./ValueDisplay.svelte";
  import ValueEditor from "./ValueEditor.svelte";
  import { findRenderer } from "./registry.ts";
  import { defaultValue } from "./default-value.ts";
  import * as DropdownMenu from "../components/ui/dropdown-menu/index.js";
  import { Button } from "../components/ui/button/index.js";
  let {
    descriptor,
    intent,
    onchange,
    readonly = false,
    error = "",
    onerror = () => {},
    direct = false,
    current,
  }: {
    descriptor: FieldDescriptor;
    intent: FieldIntent;
    onchange: (intent: FieldIntent) => void;
    readonly?: boolean;
    error?: string;
    onerror?: (message: string) => void;
    direct?: boolean;
    current?: WireValue;
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
  let shown = $derived(
    intent.mode === "value"
      ? intent.value
      : intent.mode === "omit" && current !== null && current !== undefined
        ? current
        : defaultValue(descriptor.shape),
  );
  function mode(next: string) {
    if (readonly || unknownCodec) return;
    onerror("");
    onchange(
      next === "value"
        ? {
            mode: "value",
            value:
              direct && current !== null && current !== undefined
                ? current
                : defaultValue(descriptor.shape),
          }
        : { mode: next as "omit" | "null" | "remove" },
    );
  }
</script>

<fieldset
  class="resource-field space-y-2 rounded-lg border p-3"
  class:border-primary={direct && intent.mode !== "omit"}
>
  {#if direct}
    <legend class="px-1 text-sm font-medium"
      >{descriptor.name.replaceAll("_", " ")}</legend
    >
    {#if unknownCodec}<p class="text-sm text-muted-foreground">
        No safe editor for {descriptor.codec?.name} v{descriptor.codec
          ?.version}. Its value stays unchanged.
      </p>
      <div class="break-all text-sm">
        <ValueDisplay {descriptor} value={current} />
      </div>{:else}
      <div class="flex items-center justify-between gap-2">
        <span class="text-xs text-muted-foreground">
          {intent.mode === "omit" ? "Unchanged" : "Edited"}
        </span>
        <DropdownMenu.Root>
          <DropdownMenu.Trigger
            disabled={readonly}
            aria-label={`${descriptor.name} options`}
            class="rounded-md px-2 py-1 text-sm text-muted-foreground hover:bg-muted"
            >···</DropdownMenu.Trigger
          >
          <DropdownMenu.Content align="end">
            <DropdownMenu.Item disabled={readonly} onclick={() => mode("omit")}
              >Leave unchanged</DropdownMenu.Item
            >
            <DropdownMenu.Item disabled={readonly} onclick={() => mode("value")}
              >Set a value</DropdownMenu.Item
            >
            {#if nullable}<DropdownMenu.Item
                disabled={readonly}
                onclick={() => mode("null")}>Set null</DropdownMenu.Item
              >{/if}
            {#if optional}<DropdownMenu.Item
                disabled={readonly}
                onclick={() => mode("remove")}>Remove field</DropdownMenu.Item
              >{/if}
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </div>
      {#if intent.mode === "omit" && nullable && current === null}<p
          class="text-xs text-muted-foreground"
        >
          Current value: null
        </p>{:else if intent.mode === "omit" && optional && current === undefined}<p
          class="text-xs text-muted-foreground"
        >
          Current value: not set
        </p>{/if}
      {#if intent.mode === "null" || intent.mode === "remove"}<p
          class="text-sm text-muted-foreground"
        >
          {intent.mode === "null" ? "Set to null" : "Remove this field"}
        </p>
        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled={readonly}
          onclick={() => mode("value")}>Enter a value</Button
        >{:else}<ValueEditor
          shape={descriptor.shape}
          codec={descriptor.codec}
          codecWrappers={descriptor.codec_wrappers}
          value={shown}
          onchange={(value) => onchange({ mode: "value", value })}
          label={descriptor.name}
          {readonly}
          {onerror}
          {direct}
          showLabel={false}
        />{/if}
    {/if}
    {#if error}<p role="alert">{error}</p>{/if}
  {:else}
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
  {/if}
</fieldset>
