<script lang="ts">
  import { onDestroy } from "svelte";
  import RegistryScalarHarnessRenderer from "./RegistryScalarHarnessRenderer.svelte";
  import {
    registerRenderer,
    findRenderer,
    rendererLayout,
  } from "../../src/lib/renderers/registry.ts";
  import ResourceForm from "../../src/lib/resources/ResourceForm.svelte";
  import FilterBuilder from "../../src/lib/filters/FilterBuilder.svelte";
  import type { FilterDraft } from "../../src/lib/filters/types.ts";
  import { queryFromDraft } from "../../src/lib/filters/translation.ts";
  import ActionForm from "../../src/lib/resources/ActionForm.svelte";
  import ValueDisplay from "../../src/lib/renderers/ValueDisplay.svelte";
  import ValueEditor from "../../src/lib/renderers/ValueEditor.svelte";
  import { stringifyWire } from "../../src/lib/client/codec.ts";
  import type {
    ResourceDescriptor,
    ActionInput,
  } from "../../src/lib/client/types.ts";
  const lifecycleCodec = { name: "probe.registry", version: 1 };
  const oldUnregister = registerRenderer(
    lifecycleCodec,
    RegistryScalarHarnessRenderer,
  );
  oldUnregister();
  const unregister = registerRenderer(
    lifecycleCodec,
    RegistryScalarHarnessRenderer,
    { layout: "inline" },
  );
  oldUnregister();
  const registrationRetained =
    findRenderer(lifecycleCodec) === RegistryScalarHarnessRenderer &&
    rendererLayout(lifecycleCodec) === "inline";
  let invalidLayoutRejected = false;
  try {
    registerRenderer(
      { name: "probe.invalid", version: 1 },
      RegistryScalarHarnessRenderer,
      JSON.parse('{"layout":"invalid"}'),
    );
  } catch {
    invalidLayoutRejected = true;
  }
  onDestroy(unregister);
  const labels = { queued: "Waiting", running: "Active", paused: "Active" };
  const enumShape = {
    type: "enum" as const,
    value: ["queued", "running", "paused"],
  };
  const choices = {
    name: "choices",
    shape: { type: "list" as const, value: enumShape },
    enum_labels: labels,
  };
  const descriptor: ResourceDescriptor = {
    kind: "generic-sortable",
    version: 1,
    fields: [
      choices,
      { name: "phase", shape: enumShape, enum_labels: labels },
      { name: "dictionary", shape: { type: "map", value: { type: "u64" } } },
      { name: "entries", shape: { type: "list", value: { type: "u64" } } },
      {
        name: "amounts",
        shape: { type: "list", value: { type: "string" } },
        codec: { name: "rom.decimal", version: 1 },
        codec_wrappers: ["list"],
      },
    ],
    actions: [],
    action_inputs: [],
  };
  const action: ActionInput = {
    name: "reorder",
    version: 1,
    input: {
      type: "object",
      value: [
        { name: "entries", shape: { type: "list", value: { type: "u64" } } },
      ],
    },
  };
  let filters = $state<FilterDraft>({
    rules: {
      glue: "and",
      rules: [{ field: "entries", filter: "equal", value: [7n, 7n, 9n] }],
    },
    order: [],
    limit: "20",
  });
  let filterError = $state(""),
    filtered = $state("");
  let submitted = $state(""),
    invoked = $state(""),
    enumInvoked = $state(""),
    readonly = $state(false);
</script>

<main class="max-w-xl p-4">
  <button onclick={() => (readonly = !readonly)}>Toggle readonly</button>
  <section aria-label="Resource editor">
    <ResourceForm
      {descriptor}
      {readonly}
      value={{
        dictionary: Object.fromEntries([
          ["__proto__", 2n],
          ["constructor", 3n],
        ]),
        choices: ["running", "running"],
        phase: "queued",
        entries: [7n, 7n, 9n],
        amounts: ["1.2300", "1.2300", "9.0000"],
      }}
      direct
      submit={async (input) => {
        submitted = stringifyWire(input);
      }}
    /><output data-testid="submitted">{submitted}</output>
  </section>
  <section aria-label="Action editor">
    <ActionForm
      {descriptor}
      {action}
      {readonly}
      oninvoke={async (input) => {
        invoked = stringifyWire(input);
      }}
    /><output data-testid="invoked">{invoked}</output>
  </section>
  <section aria-label="Filter editor">
    <FilterBuilder
      {descriptor}
      draft={filters}
      onchange={(next) => (filters = next)}
      disabled={false}
      onvalidity={(error) => (filterError = error)}
    /><button
      disabled={!!filterError}
      onclick={() =>
        (filtered = stringifyWire(queryFromDraft(descriptor, filters)))}
      >Apply filters</button
    ><output data-testid="filtered">{filtered}</output>
  </section>
  <section aria-label="Readonly list">
    <ValueEditor
      shape={{ type: "list", value: { type: "string" } }}
      value={["same", "same"]}
      label="locked"
      readonly
      onchange={() => {
        throw Error("Readonly mutation");
      }}
    />
  </section>
  <section aria-label="Bounded list">
    <ValueEditor
      shape={{ type: "list", value: { type: "string" } }}
      value={Array.from({ length: 101 }, (_, i) => `item-${i}`)}
      label="bounded"
      onchange={() => {
        throw Error("Bounded mutation");
      }}
    />
  </section>
  <section aria-label="Enum action editor">
    <ActionForm
      {descriptor}
      action={{
        name: "choose",
        version: 1,
        input: { type: "object", value: [choices] },
      }}
      oninvoke={async (input) => {
        enumInvoked = stringifyWire(input);
      }}
    />
    <output data-testid="enum-invoked">{enumInvoked}</output>
  </section>
  <section aria-label="Nested enum display">
    <ValueDisplay
      descriptor={{
        name: "nested",
        shape: {
          type: "optional",
          value: {
            type: "nullable",
            value: { type: "list", value: { type: "map", value: enumShape } },
          },
        },
        enum_labels: labels,
      }}
      value={[{ first: "queued", second: "paused", unknown: "retired" }]}
    />
  </section>
  <section aria-label="Registry lifecycle">
    <output data-testid="registration-retained"
      >{String(registrationRetained)}</output
    >
    <output data-testid="invalid-layout">{String(invalidLayoutRejected)}</output
    >
    <ResourceForm
      descriptor={{
        kind: "registration",
        version: 1,
        fields: [
          { name: "scalar", shape: { type: "string" }, codec: lifecycleCodec },
          {
            name: "collection",
            shape: { type: "list", value: { type: "string" } },
            codec: lifecycleCodec,
            codec_wrappers: ["list"],
          },
        ],
        actions: [],
        action_inputs: [],
      }}
      value={{ scalar: "inline", collection: ["expanded"] }}
      direct
      submit={async () => {}}
    />
  </section>
  <section aria-label="Unknown enum">
    <ResourceForm
      descriptor={{
        kind: "unknown",
        version: 1,
        fields: [
          { name: "phase", shape: enumShape, enum_labels: labels },
          { name: "note", shape: { type: "string" } },
        ],
        actions: [],
        action_inputs: [],
      }}
      value={{ phase: "retired", note: "original" }}
      direct
      submit={async () => {
        throw Error("Invalid enum submitted");
      }}
    />
  </section>
</main>
