<script lang="ts">
  import { tick, untrack } from "svelte";
  import {
    dragHandleZone,
    dragHandle,
    SHADOW_ITEM_MARKER_PROPERTY_NAME,
    type DndEvent,
  } from "svelte-dnd-action";
  import type { Shape, WireValue, FieldDescriptor } from "../client/types.ts";
  import { Button } from "../components/ui/button/index.js";
  import EnumListChoices from "./EnumListChoices.svelte";
  import ValueEditor from "./ValueEditor.svelte";
  import { defaultValue } from "./default-value.ts";
  import { previewValue } from "./value-format.ts";
  import {
    maxCollectionDepth,
    maxCollectionItems,
  } from "./collection-limits.ts";
  import XIcon from "@lucide/svelte/icons/x";
  import ArrowUpIcon from "@lucide/svelte/icons/arrow-up";
  import ArrowDownIcon from "@lucide/svelte/icons/arrow-down";
  import GripVerticalIcon from "@lucide/svelte/icons/grip-vertical";

  import type { EditorDraft, ListEditorRow as Row } from "./editor-draft.ts";
  let localDraft = $state<EditorDraft>({});
  let {
    draft = $bindable(localDraft),
    shape,
    value,
    onchange,
    onerror = () => {},
    label,
    displayLabel,
    readonly = false,
    depth = 0,
    direct = false,
    codec,
    codecWrappers = [],
    enumLabels,
  }: {
    draft?: EditorDraft;
    shape: Shape;
    value: WireValue;
    onchange: (value: WireValue) => void;
    onerror?: (message: string) => void;
    label: string;
    displayLabel?: string;
    readonly?: boolean;
    depth?: number;
    direct?: boolean;
    enumLabels?: FieldDescriptor["enum_labels"];
    codec?: FieldDescriptor["codec"];
    codecWrappers?: NonNullable<FieldDescriptor["codec_wrappers"]>;
  } = $props();
  const zoneId = $props.id();
  let nextId = 0;
  let rows = $state<Row[]>(untrack(() => draft.rows ?? []));
  let dragRows = $state<Row[]>([]);
  let list: HTMLOListElement | undefined = $state();
  const fieldLabel = $derived(displayLabel || label);
  const bounded = $derived(
    depth >= maxCollectionDepth ||
      (Array.isArray(value) && value.length > maxCollectionItems),
  );

  // An emitted order already changed rows. Prop reflection keeps those identities.
  // An external value update uses its position; no value equality can identify duplicates.
  $effect(() => {
    const incoming = Array.isArray(value) ? value : [];
    untrack(() => {
      rows = incoming.slice(0, maxCollectionItems).map((item, index) => {
        const identity = rows[index]?.identity ?? `${zoneId}-${nextId++}`;
        return {
          id: identity,
          identity,
          value: item,
          error: rows[index]?.error ?? "",
          draft: rows[index]?.draft ?? {},
        };
      });
      dragRows = rows;
      draft.rows = rows;
    });
  });

  function report() {
    draft.rows = rows;
    onerror(rows.find((row) => row.error)?.error ?? "");
  }
  function emit() {
    report();
    onchange(rows.map((row) => row.value));
  }
  function change(id: string, next: WireValue) {
    if (readonly || bounded) return;
    rows = rows.map((row) => (row.id === id ? { ...row, value: next } : row));
    dragRows = rows;
    emit();
  }
  function error(id: string, message: string) {
    if (rows.find((row) => row.id === id)?.error === message) return;
    const row = rows.find((row) => row.id === id);
    if (row) row.error = message;
    // Renderer mount/error callbacks must not replace the action's shadow order.
    report();
  }
  function setDraft(identity: string, next: EditorDraft) {
    const row = rows.find((row) => row.identity === identity);
    if (row) row.draft = next;
    const considered = dragRows.find((row) => row.identity === identity);
    if (considered) considered.draft = next;
    draft.rows = rows;
  }
  async function focus(id: string) {
    await tick();
    const row = Array.from(list?.children ?? []).find(
      (node) => (node as HTMLElement).dataset.listRowId === id,
    );
    const editor =
      row?.querySelector<HTMLElement>("input, textarea, select") ??
      row?.querySelector<HTMLElement>("button:not(:disabled)");
    editor?.focus();
  }
  async function move(id: string, delta: number) {
    if (readonly || bounded) return;
    const index = rows.findIndex((row) => row.id === id),
      target = index + delta;
    if (index < 0 || target < 0 || target >= rows.length) return;
    const copy = [...rows];
    [copy[index], copy[target]] = [copy[target], copy[index]];
    rows = copy;
    dragRows = copy;
    emit();
    await focus(id);
  }
  function remove(id: string) {
    if (readonly || bounded) return;
    rows = rows.filter((row) => row.id !== id);
    dragRows = rows;
    emit();
  }
  function append(members: string[]) {
    if (
      readonly ||
      bounded ||
      rows.length + members.length > maxCollectionItems
    )
      return;
    rows = [
      ...rows,
      ...members.map((value) => {
        const identity = `${zoneId}-${nextId++}`;
        return { id: identity, identity, value, error: "", draft: {} };
      }),
    ];
    dragRows = rows;
    emit();
  }
  function add() {
    if (readonly || bounded || rows.length >= maxCollectionItems) return;
    const identity = `${zoneId}-${nextId++}`;
    rows = [
      ...rows,
      {
        id: identity,
        identity,
        value: defaultValue(shape),
        error: "",
        draft: {},
      },
    ];
    dragRows = rows;
    emit();
  }
  function consider(event: CustomEvent<DndEvent<Row>>) {
    if (!readonly && !bounded) dragRows = event.detail.items;
  }
  function finalize(event: CustomEvent<DndEvent<Row>>) {
    if (readonly || bounded) return;
    const incoming = event.detail.items.filter(
      (row) => !(SHADOW_ITEM_MARKER_PROPERTY_NAME in row),
    );
    // A single list cannot gain rows from another zone or publish drag metadata.
    if (
      incoming.length !== rows.length ||
      new Set(incoming.map((row) => row.id)).size !== rows.length ||
      incoming.some((row) => !rows.some((current) => current.id === row.id))
    )
      return;
    rows = incoming.map((row) =>
      rows.find((current) => current.id === row.id)!,
    );
    dragRows = rows;
    emit();
  }
</script>

{#snippet items(current: Row[])}
  {#each current as row, index (row.id)}
    <li
      data-list-row-id={row.id}
      aria-label={`${fieldLabel} item ${index + 1}`}
      class={`grid ${readonly ? "grid-cols-[auto_minmax(0,1fr)_auto]" : "grid-cols-[auto_auto_minmax(0,1fr)_auto]"} items-center gap-2 border-b border-border/50 py-2 last:border-b-0`}
    >
      {#if !readonly}<span
          use:dragHandle
          aria-label={`Drag ${fieldLabel}[${index}]`}
          class="inline-flex size-7 shrink-0 touch-none items-center justify-center rounded-md hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring"
          ><GripVerticalIcon class="size-4" /></span
        >{/if}
      <span class="text-xs text-muted-foreground">Item {index + 1}</span>
      <div class="min-w-0">
        <ValueEditor
          {shape}
          {codec}
          {enumLabels}
          {codecWrappers}
          value={row.value}
          bind:draft={() => row.draft, (next) => setDraft(row.identity, next)}
          onchange={(next) => change(row.identity, next)}
          label={`${label}[${index}]`}
          displayLabel={`${fieldLabel}[${index}]`}
          {readonly}
          {direct}
          showLabel={false}
          depth={depth + 1}
          onerror={(message) => error(row.identity, message)}
        />
      </div>
      <span class="flex items-center">
        {#if !readonly}<Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label={`Move ${fieldLabel}[${index}] up`}
            disabled={index === 0}
            onclick={() => move(row.id, -1)}
            ><ArrowUpIcon class="size-4" /></Button
          ><Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label={`Move ${fieldLabel}[${index}] down`}
            disabled={index === current.length - 1}
            onclick={() => move(row.id, 1)}
            ><ArrowDownIcon class="size-4" /></Button
          >{/if}
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label={`Remove ${fieldLabel}[${index}]`}
          disabled={readonly}
          onclick={() => remove(row.id)}><XIcon class="size-4" /></Button
        >
      </span>
    </li>
  {/each}
{/snippet}

<fieldset class="space-y-2">
  <legend class="text-xs text-muted-foreground"
    >{direct ? "Items" : `${fieldLabel} items`}</legend
  >
  {#if depth >= maxCollectionDepth}<p role="alert">
      Collection nesting limit reached.
    </p>
    <p class="break-words text-sm text-muted-foreground">
      {previewValue(value)}
    </p>
  {:else if Array.isArray(value) && value.length > maxCollectionItems}<p
      role="alert"
    >
      Collection item limit reached. Editing is unavailable.
    </p>
    <ol class="space-y-1 text-sm">
      {#each value.slice(0, 10) as item, index}<li class="break-words">
          {index + 1}. {previewValue(item)}
        </li>{/each}
    </ol>
    <p class="text-sm text-muted-foreground">{value.length - 10} more items</p>
  {:else}
    {#if readonly}<ol bind:this={list} aria-label={`${fieldLabel} items`}>
        {@render items(rows)}
      </ol>
    {:else}<ol
        bind:this={list}
        aria-label={`${fieldLabel} items`}
        use:dragHandleZone={{
          items: dragRows,
          type: zoneId,
          dropFromOthersDisabled: true,
          flipDurationMs: 0,
          delayTouchStart: 80,
        }}
        onconsider={consider}
        onfinalize={finalize}
      >
        {@render items(dragRows)}
      </ol>{/if}
    {#if !readonly && !codec && shape.type === "enum"}<EnumListChoices
        members={shape.value}
        labels={enumLabels}
        label={fieldLabel}
        remaining={maxCollectionItems - rows.length}
        onappend={append}
      />{/if}
    <Button
      type="button"
      variant="outline"
      size="sm"
      aria-label={`Add ${fieldLabel} item`}
      disabled={readonly || rows.length >= maxCollectionItems}
      onclick={add}>Add item</Button
    >
  {/if}
</fieldset>
