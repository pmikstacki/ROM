<script lang="ts">
  import type {
    ResourceDescriptor,
    QuerySpec,
    FieldIntent,
  } from "../client/types.ts";
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

<form onsubmit={apply}>
  <h2>Query</h2>
  <label
    >Filter field<select
      aria-label="Query field"
      value={fieldName}
      onchange={(event) => choose(event.currentTarget.value)}
      ><option value="">No filter</option
      >{#each descriptor.fields as item}<option value={item.name}
          >{item.name}</option
        >{/each}</select
    ></label
  >
  {#if field && value.mode === "value"}<ValueEditor
      shape={field.shape}
      codec={field.codec}
      codecWrappers={field.codec_wrappers ?? []}
      value={value.value}
      onchange={(next) => (value = { mode: "value", value: next })}
      label="Query value"
      onerror={(message) => (invalid = message)}
    />{/if}
  <label
    >Sort field<select aria-label="Query sort field" bind:value={sortFieldName}
      ><option value="">ID order</option>{#each sortFields as item}<option
          value={item.name}>{item.name}</option
        >{/each}</select
    ></label
  >
  <label
    >Sort<select
      aria-label="Query sort"
      bind:value={sort}
      disabled={!sortFieldName}
      ><option value="">No sort</option><option value="asc">Ascending</option
      ><option value="desc">Descending</option></select
    ></label
  ><label
    >Limit<input
      aria-label="Query limit"
      type="number"
      min="1"
      max="1000"
      bind:value={limit}
    /></label
  >
  {#if error}<p role="alert">{error}</p>{/if}<Button
    type="submit"
    disabled={!!invalid}>Apply query</Button
  >
</form>
