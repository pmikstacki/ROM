<script lang="ts">
  import type { Shape, WireValue, FieldDescriptor } from "../client/types.ts";
  import { normalizeValue } from "../client/codec.ts";
  import { Input } from "../components/ui/input/index.js";
  import { Button } from "../components/ui/button/index.js";
  import { enumLabel } from "../client/enum-labels.ts";
  import SelectAdapter from "./SelectAdapter.svelte";
  import CheckboxAdapter from "./CheckboxAdapter.svelte";
  import SwitchAdapter from "./SwitchAdapter.svelte";
  import ValueEditor from "./ValueEditor.svelte";
  import { defaultValue } from "./default-value.ts";
  import { findRenderer } from "./registry.ts";
  import { previewValue } from "./value-format.ts";
  import { untrack } from "svelte";
  import type { EditorDraft } from "./editor-draft.ts";
  import ReferenceField from "./ReferenceField.svelte";
  import ListEditor from "./ListEditor.svelte";
  import {
    maxCollectionDepth as maxDepth,
    maxCollectionItems as maxItems,
  } from "./collection-limits.ts";
  import XIcon from "@lucide/svelte/icons/x";
  let localDraft = $state<EditorDraft>({});
  let {
    draft = $bindable(localDraft),
    shape,
    value,
    onchange,
    label,
    readonly = false,
    depth = 0,
    codec,
    codecWrappers = [],
    enumLabels,
    onerror = () => {},
    direct = false,
    showLabel = true,
    displayLabel,
  }: {
    draft?: EditorDraft;
    shape: Shape;
    value: WireValue;
    onchange: (value: WireValue) => void;
    label: string;
    readonly?: boolean;
    depth?: number;
    enumLabels?: FieldDescriptor["enum_labels"];
    codec?: FieldDescriptor["codec"];
    codecWrappers?: NonNullable<FieldDescriptor["codec_wrappers"]>;
    onerror?: (message: string) => void;
    direct?: boolean;
    showLabel?: boolean;
    displayLabel?: string;
  } = $props();
  const invalid = $derived(draft.invalid ?? "");
  const fieldLabel = $derived(displayLabel || label);
  let childErrors = $state<Record<string, string>>(
    untrack(() => draft.errors ?? {}),
  );
  function childError(key: string, error: string) {
    childErrors = { ...childErrors, [key]: error };
    draft.errors = childErrors;
    onerror(Object.values(childErrors).find(Boolean) ?? "");
  }
  function childDraft(key: string): EditorDraft {
    return untrack(() => {
      if (!draft.children || !Object.hasOwn(draft.children, key))
        draft.children = { ...draft.children, [key]: {} };
      return draft.children[key];
    });
  }
  let newKey = $state("");
  let Custom = $derived(
    codecWrappers.length === 0 ? findRenderer(codec) : undefined,
  );
  let wrapped = $derived(codecWrappers.length > 0);
  let wrapperMismatch = $derived(wrapped && codecWrappers[0] !== shape.type);
  function scalar(text: string) {
    draft.text = text;
    try {
      if (text.length > 65536) throw Error("Field input limit reached.");
      let next: WireValue = text;
      if (shape.type === "u64" || shape.type === "i64") {
        if (!/^-?\d+$/.test(text)) throw Error("Enter an integer.");
        if (text.length > 32) throw Error("Integer input limit reached.");
        const integer = BigInt(text);
        next = integer;
      }
      if (shape.type === "f64") {
        if (!text.trim() || !Number.isFinite(Number(text)))
          throw Error("Enter a finite number.");
        next = Number(text);
      }
      next = normalizeValue(shape, next, label);
      draft.invalid = "";
      onerror("");
      onchange(next);
    } catch (error) {
      draft.invalid = error instanceof Error ? error.message : "Invalid value.";
      onerror(invalid);
    }
  }
  const unknownEnum = $derived(
    shape.type === "enum" &&
      (typeof value !== "string" || !shape.value.includes(value)),
  );
  $effect(() => {
    if (shape.type === "enum" && !codec) {
      const message = unknownEnum
        ? "Stored value is not an accepted enum member. Choose a valid value."
        : "";
      untrack(() => onerror(message));
    }
  });
  function removeKey(key: string) {
    if (!value || typeof value !== "object" || Array.isArray(value)) return;
    const copy = { ...value };
    delete copy[key];
    const errors = { ...childErrors };
    delete errors[key];
    if (draft.children) delete draft.children[key];
    childErrors = errors;
    draft.errors = errors;
    onerror(Object.values(errors).find(Boolean) ?? "");
    onchange(copy);
  }
  function mapChange(key: string, next: WireValue) {
    if (value && typeof value === "object" && !Array.isArray(value))
      onchange({ ...value, [key]: next });
  }
  function addKey() {
    if (
      shape.type === "map" &&
      value &&
      typeof value === "object" &&
      !Array.isArray(value) &&
      newKey &&
      !Object.hasOwn(value, newKey)
    ) {
      onchange({ ...value, [newKey]: defaultValue(shape.value) });
      newKey = "";
    }
  }
</script>

{#if wrapperMismatch}<p role="alert">
    Codec wrapper does not match the field shape. Editing is unavailable.
  </p>
{:else if Custom}
  <Custom
    descriptor={{ name: label, shape, codec, enum_labels: enumLabels }}
    label={fieldLabel}
    {value}
    {onchange}
    {readonly}
    {onerror}
    {draft}
    onDraftChange={(next: EditorDraft) => (draft = next)}
  />
{:else if codec && !wrapped}<p role="alert">
    No renderer is registered for codec {codec.name} version {codec.version}.
    Editing is unavailable.
  </p>
{:else if shape.type === "nullable" && depth > 0}
  {#if value === null}<div
      class="flex min-h-9 items-center justify-between gap-2"
    >
      <span class="text-sm text-muted-foreground">Null</span>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        disabled={readonly}
        aria-label={`Set ${fieldLabel} value`}
        onclick={() => {
          onerror("");
          draft.text = undefined;
          draft.invalid = "";
          onchange(defaultValue(shape.value));
        }}>Enter value</Button
      >
    </div>{:else}<div class="flex items-center gap-2">
      <div class="min-w-0 flex-1">
        <ValueEditor
          shape={shape.value}
          {value}
          {onchange}
          {label}
          displayLabel={fieldLabel}
          {readonly}
          {depth}
          bind:draft
          {onerror}
          {direct}
          showLabel={false}
          {codec}
          {enumLabels}
          codecWrappers={wrapped ? codecWrappers.slice(1) : []}
        />
      </div>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        disabled={readonly}
        aria-label={`Set ${fieldLabel} null`}
        onclick={() => {
          onerror("");
          draft.text = undefined;
          draft.invalid = "";
          onchange(null);
        }}>Null</Button
      >
    </div>{/if}
{:else if shape.type === "optional" || shape.type === "nullable"}
  <ValueEditor
    shape={shape.value}
    {value}
    {onchange}
    {label}
    displayLabel={fieldLabel}
    {readonly}
    {depth}
    bind:draft
    {onerror}
    {direct}
    {showLabel}
    {codec}
    {enumLabels}
    codecWrappers={wrapped ? codecWrappers.slice(1) : []}
  />
{:else if shape.type === "bool"}
  {#if direct}<SwitchAdapter
      label={fieldLabel}
      {showLabel}
      checked={value === true}
      disabled={readonly}
      {onchange}
    />{:else}<CheckboxAdapter
      label={`${fieldLabel} value`}
      text={showLabel ? fieldLabel : ""}
      checked={value === true}
      disabled={readonly}
      {onchange}
    />{/if}
{:else if shape.type === "enum"}
  <div class="space-y-1.5">
    {#if showLabel}<span class="text-sm">{fieldLabel}</span>{/if}
    <SelectAdapter
      label={`${fieldLabel} value`}
      value={String(value ?? "")}
      disabled={readonly}
      {onchange}
      options={shape.value.map((option) => ({
        value: option,
        label: enumLabel(option, enumLabels),
      }))}
    />
    {#if unknownEnum}<p role="alert" class="break-all text-sm">
        Stored value {String(value)} is not an accepted enum member. Choose a valid
        value.
      </p>{/if}
  </div>
{:else if shape.type === "list"}
  <ListEditor
    bind:draft
    shape={shape.value}
    {value}
    {onchange}
    {onerror}
    {label}
    displayLabel={fieldLabel}
    {readonly}
    {depth}
    {direct}
    {codec}
    {enumLabels}
    codecWrappers={wrapped ? codecWrappers.slice(1) : []}
  />
{:else if shape.type === "map"}
  <fieldset class="space-y-2">
    <legend class="text-xs text-muted-foreground">
      {direct ? "Entries" : `${fieldLabel} entries`}
    </legend>
    {#if depth >= maxDepth}<p role="alert">Collection nesting limit reached.</p>
      <p class="break-words text-sm text-muted-foreground">
        {previewValue(value)}
      </p>{:else if value && typeof value === "object" && !Array.isArray(value) && Object.keys(value).length > maxItems}<div
        class="space-y-2"
      >
        <p role="alert">
          Collection item limit reached. Editing is unavailable.
        </p>
        <dl class="space-y-1 text-sm">
          {#each Object.entries(value).slice(0, 10) as [key, item]}<div
              class="break-words"
            >
              <dt class="inline font-medium">{key}:</dt>
              <dd class="inline">{previewValue(item)}</dd>
            </div>{/each}
        </dl>
        <p class="text-sm text-muted-foreground">
          {Object.keys(value).length - 10} more entries
        </p>
      </div>{:else}
      {#each Object.entries(value && typeof value === "object" && !Array.isArray(value) ? value : {}) as [key, item] (key)}
        <div
          class="grid grid-cols-[minmax(5.5rem,0.3fr)_minmax(0,1fr)_2.25rem] items-center gap-2 border-b border-border/50 py-2 last:border-b-0"
        >
          <span class="break-all text-xs text-muted-foreground">{key}</span>
          <div class="min-w-0">
            <ValueEditor
              shape={shape.value}
              {codec}
              {enumLabels}
              codecWrappers={wrapped ? codecWrappers.slice(1) : []}
              value={item}
              bind:draft={
                () => childDraft(key),
                (next) => {
                  draft.children![key] = next;
                }
              }
              onchange={(next) => mapChange(key, next)}
              label={`${label}.${key}`}
              displayLabel={`${fieldLabel}.${key}`}
              {readonly}
              {direct}
              showLabel={false}
              depth={depth + 1}
              onerror={(error) => childError(key, error)}
            />
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label={`Remove ${fieldLabel}.${key}`}
            disabled={readonly}
            onclick={() => removeKey(key)}><XIcon class="size-4" /></Button
          >
        </div>
      {/each}
      <label
        >{fieldLabel} new key<Input
          aria-label={`${fieldLabel} new key`}
          bind:value={newKey}
          disabled={readonly}
        /></label
      >
      <Button
        type="button"
        variant="outline"
        size="sm"
        aria-label={`Add ${fieldLabel} entry`}
        disabled={readonly ||
          !newKey ||
          (value !== null &&
            typeof value === "object" &&
            Object.keys(value).length >= maxItems)}
        onclick={addKey}>Add entry</Button
      >
    {/if}
  </fieldset>
{:else if shape.type === "reference"}
  <ReferenceField
    kind={shape.value.kind}
    path={label}
    {value}
    label={fieldLabel}
    {showLabel}
    {readonly}
    {onchange}
    {onerror}
  />
{:else}
  <label
    >{#if showLabel}<span>{fieldLabel}</span>{/if}<Input
      aria-label={`${fieldLabel} value`}
      type="text"
      inputmode={shape.type === "u64" || shape.type === "i64"
        ? "numeric"
        : shape.type === "f64"
          ? "decimal"
          : undefined}
      value={draft.text ?? (value === null ? "" : String(value))}
      disabled={readonly}
      aria-invalid={!!invalid}
      oninput={(event) => scalar(event.currentTarget.value)}
    /></label
  >
{/if}
{#if invalid}<p role="alert">{invalid}</p>{/if}
