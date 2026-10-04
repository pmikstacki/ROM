<script lang="ts">
  import type { Shape, WireValue, FieldDescriptor } from "../client/types.ts";
  import { normalizeValue } from "../client/codec.ts";
  import { Input } from "../components/ui/input/index.js";
  import { Button } from "../components/ui/button/index.js";
  import SelectAdapter from "./SelectAdapter.svelte";
  import CheckboxAdapter from "./CheckboxAdapter.svelte";
  import SwitchAdapter from "./SwitchAdapter.svelte";
  import ValueEditor from "./ValueEditor.svelte";
  import { defaultValue } from "./default-value.ts";
  import { findRenderer } from "./registry.ts";
  import { previewValue } from "./value-format.ts";
  import XIcon from "@lucide/svelte/icons/x";
  let {
    shape,
    value,
    onchange,
    label,
    readonly = false,
    depth = 0,
    codec,
    codecWrappers = [],
    onerror = () => {},
    direct = false,
    showLabel = true,
  }: {
    shape: Shape;
    value: WireValue;
    onchange: (value: WireValue) => void;
    label: string;
    readonly?: boolean;
    depth?: number;
    codec?: FieldDescriptor["codec"];
    codecWrappers?: NonNullable<FieldDescriptor["codec_wrappers"]>;
    onerror?: (message: string) => void;
    direct?: boolean;
    showLabel?: boolean;
  } = $props();
  let invalid = $state("");
  let childErrors = $state<Record<string, string>>({});
  let itemKeys = $state<number[]>([]);
  let nextKey = 0;
  $effect(() => {
    if (
      shape.type === "list" &&
      Array.isArray(value) &&
      itemKeys.length < value.length
    ) {
      itemKeys = [
        ...itemKeys,
        ...Array.from(
          { length: value.length - itemKeys.length },
          () => nextKey++,
        ),
      ];
    }
  });
  function childError(key: string, error: string) {
    childErrors = { ...childErrors, [key]: error };
    onerror(Object.values(childErrors).find(Boolean) ?? "");
  }
  let newKey = $state("");
  let Custom = $derived(
    codecWrappers.length === 0 ? findRenderer(codec) : undefined,
  );
  let wrapped = $derived(codecWrappers.length > 0);
  let wrapperMismatch = $derived(wrapped && codecWrappers[0] !== shape.type);
  const maxDepth = 6,
    maxItems = 100;
  function scalar(text: string) {
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
      invalid = "";
      onerror("");
      onchange(next);
    } catch (error) {
      invalid = error instanceof Error ? error.message : "Invalid value.";
      onerror(invalid);
    }
  }
  function listChange(index: number, next: WireValue) {
    if (Array.isArray(value)) {
      const copy = [...value];
      copy[index] = next;
      onchange(copy);
    }
  }
  function addList() {
    if (shape.type === "list") {
      itemKeys = [...itemKeys, nextKey++];
      onchange([
        ...(Array.isArray(value) ? value : []),
        defaultValue(shape.value),
      ]);
    }
  }
  function removeList(index: number) {
    if (!Array.isArray(value)) return;
    itemKeys = itemKeys.filter((_, i) => i !== index);
    const errors: Record<string, string> = {};
    for (const [key, error] of Object.entries(childErrors)) {
      const old = Number(key);
      if (old !== index) errors[String(old > index ? old - 1 : old)] = error;
    }
    childErrors = errors;
    onerror(Object.values(errors).find(Boolean) ?? "");
    onchange(value.filter((_, i) => i !== index));
  }
  function removeKey(key: string) {
    if (!value || typeof value !== "object" || Array.isArray(value)) return;
    const copy = { ...value };
    delete copy[key];
    const errors = { ...childErrors };
    delete errors[key];
    childErrors = errors;
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
    descriptor={{ name: label, shape, codec }}
    {value}
    {onchange}
    {readonly}
    {onerror}
  />
{:else if codec && !wrapped}<p role="alert">
    No renderer is registered for codec {codec.name} version {codec.version}.
    Editing is unavailable.
  </p>
{:else if shape.type === "nullable" && depth > 0}
  {#if value === null}<div class="flex min-h-9 items-center justify-between gap-2">
      <span class="text-sm text-muted-foreground">Null</span>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        disabled={readonly}
        aria-label={`Set ${label} value`}
        onclick={() => {
          onerror("");
          onchange(defaultValue(shape.value));
        }}>Enter value</Button
      >
    </div>{:else}<div class="flex items-center gap-2">
      <div class="min-w-0 flex-1"><ValueEditor
          shape={shape.value}
          {value}
          {onchange}
          {label}
          {readonly}
          {depth}
          {onerror}
          {direct}
          showLabel={false}
          {codec}
          codecWrappers={wrapped ? codecWrappers.slice(1) : []}
        /></div>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        disabled={readonly}
        aria-label={`Set ${label} null`}
        onclick={() => {
          onerror("");
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
    {readonly}
    {depth}
    {onerror}
    {direct}
    {showLabel}
    {codec}
    codecWrappers={wrapped ? codecWrappers.slice(1) : []}
  />
{:else if shape.type === "bool"}
  {#if direct}<SwitchAdapter
      {label}
      {showLabel}
      checked={value === true}
      disabled={readonly}
      {onchange}
    />{:else}<CheckboxAdapter
      label={`${label} value`}
      text={showLabel ? label : ""}
      checked={value === true}
      disabled={readonly}
      {onchange}
    />{/if}
{:else if shape.type === "enum"}
  <div class="space-y-1.5">
    {#if showLabel}<span class="text-sm">{label}</span>{/if}
    <SelectAdapter
      label={`${label} value`}
      value={String(value ?? "")}
      disabled={readonly}
      {onchange}
      options={shape.value.map((option) => ({ value: option, label: option }))}
    />
  </div>
{:else if shape.type === "list"}
  <fieldset class="space-y-2">
    <legend class="text-xs text-muted-foreground">
      {direct ? "Items" : `${label} items`}
    </legend>
    {#if depth >= maxDepth}<p role="alert">
        Collection nesting limit reached.
      </p><p class="break-words text-sm text-muted-foreground"
        >{previewValue(value)}</p
      >{:else if Array.isArray(value) && value.length > maxItems}<div class="space-y-2">
        <p role="alert">Collection item limit reached. Editing is unavailable.</p>
        <ol class="space-y-1 text-sm">
          {#each value.slice(0, 10) as item, index}<li class="break-words"
              >{index + 1}. {previewValue(item)}</li
            >{/each}
        </ol>
        <p class="text-sm text-muted-foreground"
          >{value.length - 10} more items</p
        >
      </div>{:else}
      {#each Array.isArray(value) ? value : [] as item, index (itemKeys[index] ?? `initial-${index}`)}
        <div class="grid grid-cols-[minmax(5.5rem,0.3fr)_minmax(0,1fr)_2.25rem] items-center gap-2 border-b border-border/50 py-2 last:border-b-0">
          <span class="text-xs text-muted-foreground">Item {index + 1}</span>
          <div class="min-w-0"><ValueEditor
            shape={shape.value}
            {codec}
            codecWrappers={wrapped ? codecWrappers.slice(1) : []}
            value={item}
            onchange={(next) => listChange(index, next)}
            label={`${label}[${index}]`}
            {readonly}
            {direct}
            showLabel={false}
            depth={depth + 1}
            onerror={(error) => childError(String(index), error)}
          /></div>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label={`Remove ${label}[${index}]`}
            disabled={readonly}
            onclick={() => removeList(index)}><XIcon class="size-4" /></Button
          >
        </div>
      {/each}
      <Button
        type="button"
        variant="outline"
        size="sm"
        aria-label={`Add ${label} item`}
        disabled={readonly ||
          (Array.isArray(value) && value.length >= maxItems)}
        onclick={addList}>Add item</Button
      >
    {/if}
  </fieldset>
{:else if shape.type === "map"}
  <fieldset class="space-y-2">
    <legend class="text-xs text-muted-foreground">
      {direct ? "Entries" : `${label} entries`}
    </legend>
    {#if depth >= maxDepth}<p role="alert">
        Collection nesting limit reached.
      </p><p class="break-words text-sm text-muted-foreground"
        >{previewValue(value)}</p
      >{:else if value && typeof value === "object" && !Array.isArray(value) && Object.keys(value).length > maxItems}<div class="space-y-2">
        <p role="alert">Collection item limit reached. Editing is unavailable.</p>
        <dl class="space-y-1 text-sm">
          {#each Object.entries(value).slice(0, 10) as [key, item]}<div
              class="break-words"><dt class="inline font-medium">{key}:</dt>
              <dd class="inline"> {previewValue(item)}</dd></div
            >{/each}
        </dl>
        <p class="text-sm text-muted-foreground"
          >{Object.keys(value).length - 10} more entries</p
        >
      </div>{:else}
      {#each Object.entries(value && typeof value === "object" && !Array.isArray(value) ? value : {}) as [key, item] (key)}
        <div class="grid grid-cols-[minmax(5.5rem,0.3fr)_minmax(0,1fr)_2.25rem] items-center gap-2 border-b border-border/50 py-2 last:border-b-0">
          <span class="break-all text-xs text-muted-foreground">{key}</span>
          <div class="min-w-0"><ValueEditor
            shape={shape.value}
            {codec}
            codecWrappers={wrapped ? codecWrappers.slice(1) : []}
            value={item}
            onchange={(next) => mapChange(key, next)}
            label={`${label}.${key}`}
            {readonly}
            {direct}
            showLabel={false}
            depth={depth + 1}
            onerror={(error) => childError(key, error)}
          /></div>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label={`Remove ${label}.${key}`}
            disabled={readonly}
            onclick={() => removeKey(key)}><XIcon class="size-4" /></Button
          >
        </div>
      {/each}
      <label
        >{label} new key<Input
          aria-label={`${label} new key`}
          bind:value={newKey}
          disabled={readonly}
        /></label
      >
      <Button
        type="button"
        variant="outline"
        size="sm"
        aria-label={`Add ${label} entry`}
        disabled={readonly ||
          !newKey ||
          (value !== null &&
            typeof value === "object" &&
            Object.keys(value).length >= maxItems)}
        onclick={addKey}>Add entry</Button
      >
    {/if}
  </fieldset>
{:else}
  <label
    >{#if showLabel}<span>{label}</span>{/if}<Input
      aria-label={`${label} value`}
      type={shape.type === "reference" ? "search" : "text"}
      inputmode={shape.type === "u64" || shape.type === "i64"
        ? "numeric"
        : shape.type === "f64"
          ? "decimal"
          : undefined}
      value={value === null ? "" : String(value)}
      placeholder={shape.type === "reference"
        ? `Resource ID in ${shape.value.kind}`
        : undefined}
      disabled={readonly}
      aria-invalid={!!invalid}
      oninput={(event) => scalar(event.currentTarget.value)}
    /></label
  >
  {#if shape.type === "reference" && showLabel}<p class="text-xs text-muted-foreground">
      Resource ID in {shape.value.kind}
    </p>{/if}
{/if}
{#if invalid}<p role="alert">{invalid}</p>{/if}
