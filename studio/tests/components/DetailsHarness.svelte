<script lang="ts">
  import ResourcePage from "../../src/lib/application/ResourcePage.svelte";
  import { createApplication } from "../../src/lib/application/controller.ts";
  import { createClient } from "../../src/lib/client/client.ts";
  import type {
    ProjectedView,
    ResourceDescriptor,
  } from "../../src/lib/client/types.ts";
  let descriptor = $state.raw<ResourceDescriptor>({
    kind: "DraftFixture",
    version: 1,
    fields: [
      { name: "title", shape: { type: "string" } },
      { name: "enabled", shape: { type: "bool" } },
    ],
    actions: [],
    action_inputs: [],
  });
  let selected = $state.raw<ProjectedView>({
      key: { kind: "DraftFixture", id: "one" },
      revision: 1n,
      value: { title: "original", enabled: false },
    }),
    generation = $state(0);
  function reload() {
    generation++;
  }
  const runtime = createApplication(createClient({ base: "/rom-studio/api" }));
  const controller = { ...runtime, selectRow: async () => reload() };
  const snapshot = $derived({
    ...runtime.state,
    query: { filters: [{ field: "title", value: "original" }], limit: 50 },
    selected,
    rows: [selected],
  });
</script>

<main>
  <h1>Draft regression fixture</h1>
  <button
    onclick={() => (descriptor = { ...descriptor, version: 2, fields: [] })}
    >Remove field definition</button
  >
  <button
    onclick={() =>
      (selected = {
        ...selected,
        revision: 2n,
        value: { title: "other change", enabled: false },
      })}>Advance live revision</button
  >{#key generation}<ResourcePage {descriptor} {controller} {snapshot} />{/key}
</main>
