<script lang="ts">
  import type {
    FieldDescriptor,
    FieldIntent,
    WireValue,
  } from "../client/types.ts";
  import type { EditorDraft } from "./editor-draft.ts";
  import SelectAdapter from "./SelectAdapter.svelte";
  import ValueDisplay from "./ValueDisplay.svelte";
  import ValueEditor from "./ValueEditor.svelte";
  import { semanticKind } from "./semantic-fields.ts";
  import { findRenderer, rendererLayout } from "./registry.ts";
  import { baseShape, defaultValue } from "./default-value.ts";
  import * as DropdownMenu from "../components/ui/dropdown-menu/index.js";
  import * as Dialog from "../components/ui/dialog/index.js";
  import MoreHorizontalIcon from "@lucide/svelte/icons/more-horizontal";
  import InfoIcon from "@lucide/svelte/icons/info";
  let {
    descriptor,
    intent,
    onchange,
    readonly = false,
    error = "",
    onerror = () => {},
    direct = false,
    current,
    displayLabel,
    help,
    draft = $bindable<EditorDraft>({}),
  }: {
    descriptor: FieldDescriptor;
    intent: FieldIntent;
    onchange: (intent: FieldIntent) => void;
    readonly?: boolean;
    error?: string;
    onerror?: (message: string) => void;
    direct?: boolean;
    current?: WireValue;
    displayLabel?: string;
    help?: string;
    draft?: EditorDraft;
  } = $props();
  const controlLabel = $derived(displayLabel || descriptor.name);
  const helpId = $props.id();
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
  let collection = $derived(
    baseShape(descriptor.shape).type === "list" ||
      baseShape(descriptor.shape).type === "map",
  );
  const standardControl = $derived(
    !!semanticKind(descriptor.codec) &&
      (baseShape(descriptor.shape).type === "string" ||
        (semanticKind(descriptor.codec) === "unit-value" &&
          baseShape(descriptor.shape).type === "map")),
  );
  let expandedControl = $derived(
    !standardControl &&
      !(rendererLayout(descriptor.codec) === "inline" && !collection) &&
      (collection || !!descriptor.codec),
  );
  let expandedLabel = $derived(
    Array.isArray(shown)
      ? `${shown.length} ${shown.length === 1 ? "item" : "items"} · ${readonly ? "View" : "Edit"}`
      : shown !== null && typeof shown === "object"
        ? `${Object.keys(shown).length} ${Object.keys(shown).length === 1 ? "entry" : "entries"} · ${readonly ? "View" : "Edit"}`
        : `${readonly ? "View" : "Edit"} value`,
  );
  let expandedOpen = $state(false);
  let editorGeneration = $state(0);
  function mode(next: string) {
    if (readonly || unknownCodec) return;
    draft = {};
    if (direct) editorGeneration += 1;
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

{#snippet valueControl()}
  {#if expandedControl}
    <Dialog.Root bind:open={expandedOpen}>
      <Dialog.Trigger
        class="inline-flex h-9 max-w-full items-center truncate rounded-md px-2 text-sm text-primary hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring"
        aria-label={`${readonly ? "View" : "Edit"} ${controlLabel}`}
        >{expandedLabel}</Dialog.Trigger
      >
      <Dialog.Content class="max-h-[80dvh] overflow-y-auto sm:max-w-xl">
        <Dialog.Header>
          <Dialog.Title>{controlLabel.replaceAll("_", " ")}</Dialog.Title>
          <Dialog.Description>
            {readonly
              ? "This value is read-only while editing is unavailable."
              : "Changes stay in the draft until you submit the form."}
          </Dialog.Description>
        </Dialog.Header>
        {#if readonly}<div class="break-all text-sm">
            <ValueDisplay {descriptor} value={shown} />
          </div>{:else}{#key editorGeneration}<ValueEditor
              bind:draft
              shape={descriptor.shape}
              codec={descriptor.codec}
              enumLabels={descriptor.enum_labels}
              codecWrappers={descriptor.codec_wrappers}
              value={shown}
              onchange={(value) => onchange({ mode: "value", value })}
              label={descriptor.name}
              displayLabel={controlLabel}
              {readonly}
              {onerror}
              {direct}
              showLabel={false}
            />{/key}{/if}
      </Dialog.Content>
    </Dialog.Root>
  {:else}
    {#key editorGeneration}<ValueEditor
        bind:draft
        shape={descriptor.shape}
        codec={descriptor.codec}
        enumLabels={descriptor.enum_labels}
        codecWrappers={descriptor.codec_wrappers}
        value={shown}
        onchange={(value) => onchange({ mode: "value", value })}
        label={descriptor.name}
        displayLabel={controlLabel}
        {readonly}
        {onerror}
        {direct}
        showLabel={false}
      />{/key}
  {/if}
{/snippet}

<div
  role="group"
  aria-label={controlLabel.replaceAll("_", " ")}
  aria-describedby={help ? helpId : undefined}
  class="resource-field grid grid-cols-[minmax(6.5rem,0.34fr)_minmax(0,1fr)_2.25rem] items-center gap-x-2 gap-y-1 border-b border-border/60 py-2.5 last:border-b-0"
>
  <span class="min-w-0 break-words text-sm font-medium" title={help}
    >{displayLabel || descriptor.name.replaceAll("_", " ")}</span
  >
  <div class="col-start-2 row-start-1 min-w-0">
    {#if unknownCodec}<div
        class="truncate text-sm text-muted-foreground"
        title={`No safe editor for ${descriptor.codec?.name} v${descriptor.codec?.version}. Its value stays unchanged.`}
      >
        <ValueDisplay {descriptor} value={current} />
      </div>{:else if intent.mode === "null"}<span
        class="text-sm text-muted-foreground">Set to null</span
      >{:else if intent.mode === "remove"}<span
        class="text-sm text-muted-foreground">Remove this field</span
      >{:else if direct && intent.mode === "omit" && nullable && current === null}<span
        class="text-sm text-muted-foreground">Current value: null</span
      >{:else if direct && intent.mode === "omit" && optional && current === undefined}<span
        class="text-sm text-muted-foreground">Current value: not set</span
      >{:else if !direct && intent.mode === "omit"}<span
        class="text-sm text-muted-foreground">Unchanged / omitted</span
      >{:else}{@render valueControl()}{/if}
  </div>
  <div class="col-start-3 row-start-1 justify-self-end">
    {#if unknownCodec}<Dialog.Root>
        <Dialog.Trigger
          aria-label={`${controlLabel} read-only details`}
          class="inline-flex size-9 items-center justify-center rounded-md text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring"
          ><InfoIcon class="size-4" /></Dialog.Trigger
        >
        <Dialog.Content class="max-h-[80dvh] overflow-y-auto sm:max-w-xl">
          <Dialog.Header>
            <Dialog.Title>{controlLabel.replaceAll("_", " ")}</Dialog.Title>
            <Dialog.Description>
              No safe editor for {descriptor.codec?.name} v{descriptor.codec
                ?.version}. Its value stays unchanged.
            </Dialog.Description>
          </Dialog.Header>
          <div class="break-all text-sm">
            <ValueDisplay {descriptor} value={current} />
          </div>
        </Dialog.Content>
      </Dialog.Root>{:else if direct}<DropdownMenu.Root>
        <DropdownMenu.Trigger
          disabled={readonly}
          aria-label={`${controlLabel} options`}
          title={intent.mode === "omit" ? "Unchanged" : "Edited"}
          class="relative inline-flex size-9 items-center justify-center rounded-md text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
          ><MoreHorizontalIcon
            class="size-4"
          />{#if intent.mode !== "omit"}<span
              aria-hidden="true"
              class="absolute right-0 top-0 size-1.5 rounded-full bg-primary"
            ></span>{/if}</DropdownMenu.Trigger
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
      </DropdownMenu.Root>{:else}<SelectAdapter
        label={`${controlLabel} mode`}
        value={intent.mode}
        disabled={readonly}
        onchange={mode}
        compact
        options={[
          { value: "omit", label: "Unchanged / omitted" },
          { value: "value", label: "Set value" },
          ...(nullable ? [{ value: "null", label: "Set null" }] : []),
          ...(optional ? [{ value: "remove", label: "Remove value" }] : []),
        ]}
      />{/if}
  </div>
  {#if help}<span id={helpId} class="sr-only">{help}</span>{/if}
  {#if error}<p role="alert" class="col-span-3 text-sm text-destructive">
      {error}
    </p>{/if}
</div>
