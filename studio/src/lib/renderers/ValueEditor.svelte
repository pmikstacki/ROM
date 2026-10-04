<script lang="ts">
  import type { Shape, WireValue, FieldDescriptor } from "../client/types.ts";
  import { normalizeValue } from "../client/codec.ts";
  import { Input } from "../components/ui/input/index.js";
  import { Button } from "../components/ui/button/index.js";
  import SelectAdapter from "./SelectAdapter.svelte";
  import CheckboxAdapter from "./CheckboxAdapter.svelte";
  import ValueEditor from "./ValueEditor.svelte";
  import { defaultValue } from "./default-value.ts";
  import { findRenderer } from "./registry.ts";
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
{:else if shape.type === "optional" || shape.type === "nullable"}
  <ValueEditor
    shape={shape.value}
    {value}
    {onchange}
    {label}
    {readonly}
    {depth}
    {onerror}
    {codec}
    codecWrappers={wrapped ? codecWrappers.slice(1) : []}
  />
{:else if shape.type === "bool"}
  <CheckboxAdapter
    label={`${label} value`}
    text={label}
    checked={value === true}
    disabled={readonly}
    {onchange}
  />
{:else if shape.type === "enum"}
  <div class="space-y-1.5">
    <span class="text-sm">{label}</span>
    <SelectAdapter
      label={`${label} value`}
      value={String(value ?? "")}
      disabled={readonly}
      {onchange}
      options={shape.value.map((option) => ({ value: option, label: option }))}
    />
  </div>
{:else if shape.type === "list"}
  <fieldset>
    <legend>{label} items</legend>
    {#if depth >= maxDepth}<p role="alert">
        Collection nesting limit reached.
      </p>{:else if Array.isArray(value) && value.length > maxItems}<p
        role="alert"
      >
        Collection item limit reached. Editing is unavailable.
      </p>{:else}
      {#each Array.isArray(value) ? value : [] as item, index (itemKeys[index] ?? `initial-${index}`)}
        <ValueEditor
          shape={shape.value}
          {codec}
          codecWrappers={wrapped ? codecWrappers.slice(1) : []}
          value={item}
          onchange={(next) => listChange(index, next)}
          label={`${label}[${index}]`}
          {readonly}
          depth={depth + 1}
          onerror={(error) => childError(String(index), error)}
        />
        <Button
          type="button"
          disabled={readonly}
          onclick={() => removeList(index)}>Remove {label}[{index}]</Button
        >
      {/each}
      <Button
        type="button"
        disabled={readonly ||
          (Array.isArray(value) && value.length >= maxItems)}
        onclick={addList}>Add {label} item</Button
      >
    {/if}
  </fieldset>
{:else if shape.type === "map"}
  <fieldset>
    <legend>{label} entries</legend>
    {#if depth >= maxDepth}<p role="alert">
        Collection nesting limit reached.
      </p>{:else if value && typeof value === "object" && !Array.isArray(value) && Object.keys(value).length > maxItems}<p
        role="alert"
      >
        Collection item limit reached. Editing is unavailable.
      </p>{:else}
      {#each Object.entries(value && typeof value === "object" && !Array.isArray(value) ? value : {}) as [key, item] (key)}
        <ValueEditor
          shape={shape.value}
          {codec}
          codecWrappers={wrapped ? codecWrappers.slice(1) : []}
          value={item}
          onchange={(next) => mapChange(key, next)}
          label={`${label}.${key}`}
          {readonly}
          depth={depth + 1}
          onerror={(error) => childError(key, error)}
        />
        <Button type="button" disabled={readonly} onclick={() => removeKey(key)}
          >Remove {label}.{key}</Button
        >
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
        disabled={readonly ||
          !newKey ||
          (value !== null &&
            typeof value === "object" &&
            Object.keys(value).length >= maxItems)}
        onclick={addKey}>Add {label} entry</Button
      >
    {/if}
  </fieldset>
{:else}
  <label
    >{label}<Input
      aria-label={`${label} value`}
      value={value === null ? "" : String(value)}
      disabled={readonly}
      aria-invalid={!!invalid}
      oninput={(event) => scalar(event.currentTarget.value)}
    /></label
  >
{/if}
{#if invalid}<p role="alert">{invalid}</p>{/if}
