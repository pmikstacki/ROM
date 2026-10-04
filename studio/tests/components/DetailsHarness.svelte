<script lang="ts">
  import ResourcePage from "../../src/lib/application/ResourcePage.svelte";
  import { createApplication } from "../../src/lib/application/controller.ts";
  import { createClient } from "../../src/lib/client/client.ts";
  import type {
    ProjectedView,
    ResourceDescriptor,
  } from "../../src/lib/client/types.ts";
  const descriptor: ResourceDescriptor = {
    kind: "DraftFixture",
    version: 1,
    fields: [{ name: "title", shape: { type: "string" } }],
    actions: [],
    action_inputs: [],
  };
  let selected = $state.raw<ProjectedView>({
      key: { kind: "DraftFixture", id: "one" },
      revision: 1n,
      value: { title: "original" },
    }),
    generation = $state(0);
  function reload() {
    generation++;
  }
  const runtime = createApplication(createClient({ base: "/rom-studio/api" }));
  const controller = { ...runtime, selectRow: async () => reload() };
  const snapshot = $derived({ ...runtime.state, selected, rows: [selected] });
</script>

<main>
  <h1>Draft regression fixture</h1>
  <button
    onclick={() =>
      (selected = {
        ...selected,
        revision: 2n,
        value: { title: "other change" },
      })}>Advance live revision</button
  >{#key generation}<ResourcePage {descriptor} {controller} {snapshot} />{/key}
</main>
