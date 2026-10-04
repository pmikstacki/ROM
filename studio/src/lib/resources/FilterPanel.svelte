<script lang="ts">
  import type { ResourceDescriptor } from "../client/types.ts";
  import type { FilterDraft } from "../filters/types.ts";
  import { scalarShape } from "../filters/translation.ts";
  import FilterBuilder from "../filters/FilterBuilder.svelte";
  import SelectAdapter from "../renderers/SelectAdapter.svelte";
  import { Button } from "../components/ui/button/index.js";
  import { Input } from "../components/ui/input/index.js";
  let {
    descriptor,
    draft,
    onchange,
    disabled,
    discardDisabled = disabled,
    invalid,
    onvalidity,
    onapply,
    ondiscard,
    compact = false,
  }: {
    descriptor: ResourceDescriptor;
    draft: FilterDraft;
    onchange: (next: FilterDraft) => void;
    disabled: boolean;
    discardDisabled?: boolean;
    invalid: string;
    onvalidity: (error: string) => void;
    onapply: () => void;
    ondiscard: () => void;
    compact?: boolean;
  } = $props();
  const order = $derived(draft.order ?? []);
  const fields = $derived(
    descriptor.fields.filter((field) => scalarShape(field.shape)),
  );
  function changeOrder(
    index: number,
    field: string,
    direction: "asc" | "desc",
  ) {
    const next = [...order];
    next[index] = { field, direction };
    onchange({ ...draft, order: next });
  }
</script>

<form
  class="space-y-4"
  onsubmit={(event) => {
    event.preventDefault();
    if (!disabled && !invalid) onapply();
  }}
>
  <FilterBuilder
    {descriptor}
    {draft}
    {onchange}
    {disabled}
    {compact}
    {onvalidity}
  />
  <div class="space-y-3 border-t pt-4">
    <h3 class="text-xs font-semibold text-muted-foreground">Sorting</h3>
    {#each order as item, index (index)}
      <div class="space-y-2 rounded-md border p-2">
        <SelectAdapter
          label={`Sort ${index + 1} field`}
          value={item.field}
          options={fields.map((field) => ({
            value: field.name,
            label: field.name,
          }))}
          onchange={(field) => changeOrder(index, field, item.direction)}
          {disabled}
        />
        <SelectAdapter
          label={`Sort ${index + 1} direction`}
          value={item.direction}
          options={[
            { value: "asc", label: "Ascending" },
            { value: "desc", label: "Descending" },
          ]}
          onchange={(direction) =>
            changeOrder(index, item.field, direction as "asc" | "desc")}
          {disabled}
        />
        <Button
          type="button"
          variant="ghost"
          size="sm"
          {disabled}
          onclick={() =>
            onchange({ ...draft, order: order.filter((_, i) => i !== index) })}
          >Remove sort {index + 1}</Button
        >
      </div>
    {/each}
    <Button
      type="button"
      variant="outline"
      size="sm"
      disabled={disabled ||
        order.length >= 4 ||
        fields.every((field) =>
          order.some((item) => item.field === field.name),
        )}
      onclick={() => {
        const field = fields.find(
          (field) => !order.some((item) => item.field === field.name),
        );
        if (field)
          onchange({
            ...draft,
            order: [...order, { field: field.name, direction: "asc" }],
          });
      }}>Add sort</Button
    >
    {#if !order.length}<p class="text-xs text-muted-foreground">
        Resource ID order
      </p>{/if}
    <label class="block space-y-1.5 text-xs font-medium"
      >Page size
      <Input
        aria-label="Query limit"
        type="text"
        inputmode="numeric"
        value={draft.limit}
        {disabled}
        oninput={(event) =>
          onchange({ ...draft, limit: event.currentTarget.value })}
      />
    </label>
  </div>
  <div class="flex flex-wrap gap-2 border-t pt-4">
    <Button type="submit" disabled={disabled || !!invalid}>Apply filters</Button
    >
    <Button
      type="button"
      variant="outline"
      disabled={discardDisabled}
      onclick={ondiscard}>Discard edits</Button
    >
    <Button
      type="button"
      variant="ghost"
      {disabled}
      onclick={() => onchange({ ...draft, rules: { glue: "and", rules: [] } })}
      >Clear filters</Button
    >
  </div>
</form>
