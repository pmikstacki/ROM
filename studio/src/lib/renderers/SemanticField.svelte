<script lang="ts">
  import { stringifyWire } from "../client/codec.ts";
  import type { RendererProps } from "./types.ts";
  import { semanticKind, normalizeSemantic } from "./semantic-fields.ts";
  import { Input } from "../components/ui/input/index.js";
  import { Textarea } from "../components/ui/textarea/index.js";
  import { Button } from "../components/ui/button/index.js";
  import * as Popover from "../components/ui/popover/index.js";
  import CalendarIcon from "@lucide/svelte/icons/calendar";
  import ClockIcon from "@lucide/svelte/icons/clock";
  import PaletteIcon from "@lucide/svelte/icons/palette";
  import AtSignIcon from "@lucide/svelte/icons/at-sign";
  import LinkIcon from "@lucide/svelte/icons/link";
  import AlignLeftIcon from "@lucide/svelte/icons/align-left";
  import CodeIcon from "@lucide/svelte/icons/code";
  import HashIcon from "@lucide/svelte/icons/hash";
  import ExpandIcon from "@lucide/svelte/icons/expand";
  let {
    descriptor,
    value,
    onchange,
    onerror = () => {},
    label,
    readonly = false,
    mode = "editor",
    draft: editorDraft,
    onDraftChange = () => {},
  }: RendererProps = $props();
  const name = $derived(label || descriptor.name);
  const kind = $derived(semanticKind(descriptor.codec));
  const compatible = $derived(
    kind === "unit-value"
      ? descriptor.shape.type === "map" &&
          descriptor.shape.value.type === "string"
      : descriptor.shape.type === "string",
  );
  const unitValue = $derived(
    value && typeof value === "object" && !Array.isArray(value) ? value : {},
  );
  let draft = $state(""),
    unit = $state(""),
    invalid = $state(""),
    expanded = $state(false);
  let lastSignature: string | undefined;
  function saveDraft(signature: string) {
    if (!editorDraft) return;
    onDraftChange({
      ...editorDraft,
      text: draft,
      invalid,
      state: { ...editorDraft.state, unit, sourceSignature: signature },
    });
  }
  const componentId = $props.id();
  const errorId = `${componentId}-error`;
  $effect(() => {
    const signature = stringifyWire(value);
    if (signature === lastSignature) return;
    lastSignature = signature;
    if (editorDraft?.state?.sourceSignature === signature) {
      draft = editorDraft.text ?? "";
      invalid = editorDraft.invalid ?? "";
      unit =
        typeof editorDraft.state.unit === "string"
          ? editorDraft.state.unit
          : "";
    } else {
      draft =
        typeof value === "string"
          ? value
          : typeof unitValue.value === "string"
            ? unitValue.value
            : "";
      unit = typeof unitValue.unit === "string" ? unitValue.unit : "";
      invalid = "";
      if (!readonly && mode === "editor" && compatible && kind) {
        try {
          normalizeSemantic(kind, value);
        } catch (error) {
          invalid =
            error instanceof Error ? error.message : "Invalid field value.";
        }
      }
      saveDraft(signature);
    }
    onerror(invalid);
  });
  function update(next: string, nextUnit = unit) {
    draft = next;
    unit = nextUnit;
    if (!kind || !compatible || readonly) return;
    try {
      const canonical = normalizeSemantic(
        kind,
        kind === "unit-value" ? { value: next, unit: nextUnit } : next,
      );
      invalid = "";
      onerror("");
      lastSignature = stringifyWire(canonical);
      saveDraft(lastSignature);
      onchange(canonical);
    } catch (error) {
      invalid = error instanceof Error ? error.message : "Invalid field value.";
      onerror(invalid);
      saveDraft(stringifyWire(value));
    }
  }
  const Icon = $derived(
    kind === "date" || kind === "datetime"
      ? CalendarIcon
      : kind === "time"
        ? ClockIcon
        : kind === "color"
          ? PaletteIcon
          : kind === "email"
            ? AtSignIcon
            : kind === "url"
              ? LinkIcon
              : kind === "multiline"
                ? AlignLeftIcon
                : kind === "json-document"
                  ? CodeIcon
                  : HashIcon,
  );
  const formatted = $derived(
    kind === "unit-value"
      ? `${unitValue.value ?? ""} ${unitValue.unit ?? ""}`
      : typeof value === "string"
        ? value
        : "Invalid encoded field",
  );
  const color = $derived(
    typeof value === "string" &&
      /^#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?$/.test(value)
      ? value
      : undefined,
  );
</script>

{#if readonly || mode !== "editor" || !compatible || !kind}
  <span
    class="inline-flex min-w-0 max-w-full items-center gap-2"
    title={formatted.slice(0, 512)}
  >
    {#if kind === "color" && color}<span
        class="size-3 shrink-0 rounded-sm ring-1 ring-border"
        style:background-color={color}
        aria-label={`Color ${color}`}
      ></span>{/if}
    <span class="truncate whitespace-pre-line"
      >{formatted.length > 160
        ? `${formatted.slice(0, 157)}…`
        : formatted}</span
    >
    {#if mode !== "cell" && compatible && kind && formatted.length > 160}
      <Popover.Root>
        <Popover.Trigger>
          {#snippet child({ props })}<Button
              {...props}
              variant="ghost"
              size="icon"
              aria-label={`View full ${name}`}><ExpandIcon /></Button
            >{/snippet}
        </Popover.Trigger>
        <Popover.Content class="w-[min(90vw,36rem)]" align="end">
          <label class="mb-2 block text-sm font-medium" for={`${errorId}-full`}
            >{name}</label
          >
          <Textarea
            id={`${errorId}-full`}
            aria-label={`${name} full value`}
            readonly
            class="min-h-48 max-h-96 font-mono"
            value={formatted}
          />
          <p class="mt-2 text-xs text-muted-foreground">
            Select and copy the complete stored value. This view does not edit it.
          </p>
        </Popover.Content>
      </Popover.Root>
    {/if}
  </span>
{:else}
  <div class="flex min-w-0 items-center gap-1" data-semantic-field={kind}>
    <div class="relative min-w-0 flex-1">
      <Icon
        class="pointer-events-none absolute left-2 top-2.5 size-4 text-muted-foreground"
        aria-hidden="true"
      />
      <Input
        type={kind === "date" ? "date" : "text"}
        min={kind === "date" ? "0001-01-01" : undefined}
        max={kind === "date" ? "9999-12-31" : undefined}
        value={draft}
        readonly={kind === "multiline" || kind === "json-document"}
        onclick={() => {
          if (kind === "multiline" || kind === "json-document") expanded = true;
        }}
        inputmode={kind === "decimal" || kind === "unit-value"
          ? "decimal"
          : kind === "email"
            ? "email"
            : kind === "url"
              ? "url"
              : "text"}
        aria-label={`${name} value`}
        aria-invalid={!!invalid}
        aria-describedby={invalid ? errorId : undefined}
        class="h-9 pl-8 shadow-none"
        placeholder={kind === "datetime"
          ? "YYYY-MM-DDTHH:MM:SSZ"
          : kind === "time"
            ? "HH:MM:SS"
            : kind === "color"
              ? "#RRGGBB"
              : undefined}
        oninput={(event) => update(event.currentTarget.value)}
      />
    </div>
    {#if kind === "unit-value"}<Input
        value={unit}
        class="h-9 w-12 shrink-0 px-2 shadow-none"
        aria-label={`${name} unit`}
        aria-invalid={!!invalid}
        oninput={(event) => update(draft, event.currentTarget.value)}
      />{/if}
    {#if kind === "color"}
      <label
        class="relative flex size-9 shrink-0 cursor-pointer items-center justify-center rounded-md"
        title="Choose color"
      >
        <span
          class="size-5 rounded-sm ring-1 ring-border"
          style:background-color={/^#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?$/.test(
            draft,
          )
            ? draft
            : "transparent"}
        ></span>
        <input
          type="color"
          class="absolute inset-0 cursor-pointer opacity-0"
          aria-label={`Choose ${name}`}
          value={/^#[0-9a-fA-F]{6}/.test(draft) ? draft.slice(0, 7) : "#000000"}
          oninput={(event) =>
            update(
              event.currentTarget.value +
                (draft.length === 9 ? draft.slice(7) : ""),
            )}
        />
      </label>
    {/if}
    {#if kind === "multiline" || kind === "json-document"}
      <Popover.Root bind:open={expanded}>
        <Popover.Trigger>
          {#snippet child({ props })}<Button
              {...props}
              variant="ghost"
              size="icon"
              aria-label={`Expand ${name} editor`}><ExpandIcon /></Button
            >{/snippet}
        </Popover.Trigger>
        <Popover.Content class="w-[min(90vw,36rem)]" align="end">
          <label
            class="mb-2 block text-sm font-medium"
            for={`${errorId}-editor`}>{name}</label
          >
          <Textarea
            id={`${errorId}-editor`}
            aria-label={`${name} expanded value`}
            aria-invalid={!!invalid}
            aria-describedby={invalid ? errorId : undefined}
            class="min-h-48 font-mono"
            value={draft}
            oninput={(event) => update(event.currentTarget.value)}
          />
          <p class="mt-2 text-xs text-muted-foreground">
            {kind === "json-document"
              ? "Exact JSON text is preserved, including numeric tokens."
              : "Line breaks and text are preserved."}
          </p>
        </Popover.Content>
      </Popover.Root>
    {/if}
  </div>
  {#if invalid}<p id={errorId} role="alert" class="text-xs text-destructive">
      {invalid}
    </p>{/if}
{/if}
