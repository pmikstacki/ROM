<script lang="ts">
  import type {
    ResourceDescriptor,
    QuerySpec,
    FieldIntent,
  } from "../client/types.ts";
  import SelectAdapter from "../renderers/SelectAdapter.svelte";
  import { Input } from "../components/ui/input/index.js";
  import ValueEditor from "../renderers/ValueEditor.svelte";
  import { defaultValue, baseShape } from "../renderers/default-value.ts";
  import { normalizeValue } from "../client/codec.ts";
  import { Button } from "../components/ui/button/index.js";
  let {
    descriptor,
    onchange,
  }: { descriptor: ResourceDescriptor; onchange: (query: QuerySpec) => void } =
    $props();
  let fieldName = $state("");
  let filter = $state(false);
  let sort = $state("");
  let sortFieldName = $state("");
  let limit = $state("50");
  let value = $state<FieldIntent>({ mode: "value", value: "" });
  let error = $state("");
  let invalid = $state("");
  const sortFields = $derived(
    descriptor.fields.filter(
      (item) => !["list", "map"].includes(baseShape(item.shape).type),
    ),
  );
  let field = $derived(
    descriptor.fields.find((item) => item.name === fieldName),
  );
  function choose(name: string) {
    fieldName = name;
    filter = !!name;
    if (field) value = { mode: "value", value: defaultValue(field.shape) };
    invalid = "";
  }
  function apply(event: SubmitEvent) {
    event.preventDefault();
    try {
      const pageSize = Number(limit);
      if (!Number.isInteger(pageSize) || pageSize < 1 || pageSize > 1000)
        throw Error("Limit must be between 1 and 1000.");
      const query: QuerySpec = { limit: pageSize };
      if (filter && field) {
        if (value.mode !== "value") throw Error("Set a query value.");
        query.filters = [
          {
            field: field.name,
            value: normalizeValue(field.shape, value.value, field.name),
          },
        ];
      }
      if (sort && sortFieldName)
        query.order = [
          { field: sortFieldName, direction: sort as "asc" | "desc" },
        ];
      error = "";
      onchange(query);
    } catch (problem) {
      error = problem instanceof Error ? problem.message : "Invalid query.";
    }
  }
</script>

<form
  class="query-toolbar flex flex-wrap items-end gap-3 rounded-lg border bg-card p-4"
  onsubmit={apply}
>
  <h2 class="sr-only">Query</h2>
  <div class="min-w-36 space-y-1.5">
    <span class="text-xs font-medium text-muted-foreground">Filter field</span>
    <SelectAdapter
      label="Query field"
      value={fieldName}
      onchange={choose}
      options={[
        { value: "", label: "No filter" },
        ...descriptor.fields.map((item) => ({
          value: item.name,
          label: item.name,
        })),
      ]}
    />
  </div>
  {#if field && value.mode === "value"}<div class="min-w-36">
      <ValueEditor
        shape={field.shape}
        codec={field.codec}
        codecWrappers={field.codec_wrappers ?? []}
        value={value.value}
        onchange={(next) => (value = { mode: "value", value: next })}
        label="Query value"
        onerror={(message) => (invalid = message)}
      />
    </div>{/if}
  <div class="min-w-36 space-y-1.5">
    <span class="text-xs font-medium text-muted-foreground">Sort field</span>
    <SelectAdapter
      label="Query sort field"
      value={sortFieldName}
      onchange={(next) => (sortFieldName = next)}
      options={[
        { value: "", label: "ID order" },
        ...sortFields.map((item) => ({ value: item.name, label: item.name })),
      ]}
    />
  </div>
  <div class="min-w-36 space-y-1.5">
    <span class="text-xs font-medium text-muted-foreground">Direction</span>
    <SelectAdapter
      label="Query sort"
      value={sort}
      onchange={(next) => (sort = next)}
      disabled={!sortFieldName}
      options={[
        { value: "", label: "No sort" },
        { value: "asc", label: "Ascending" },
        { value: "desc", label: "Descending" },
      ]}
    />
  </div>
  <label class="w-24 space-y-1.5 text-xs font-medium text-muted-foreground"
    >Limit<Input
      aria-label="Query limit"
      type="number"
      min="1"
      max="1000"
      bind:value={limit}
    /></label
  >
  <Button type="submit" disabled={!!invalid}>Apply query</Button>
  {#if error}<p class="w-full text-sm text-destructive" role="alert">
      {error}
    </p>{/if}
</form>
