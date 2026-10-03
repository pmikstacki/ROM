<script lang="ts">
  import ResourceDetails from "../../src/lib/application/ResourceDetails.svelte";
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
  >{#key generation}<ResourceDetails
      {descriptor}
      {selected}
      blocked={false}
      onmutate={async () => {}}
      onreload={reload}
    />{/key}
</main>
