<script lang="ts">
  import { ResourceForm, type ResourceFormSubmission } from "rom-studio/forms";
  import { stringifyWire, type ResourceDescriptor, type WireObject } from "rom-studio/client";
  const descriptor: ResourceDescriptor = {
    kind: "public-forms", version: 1, actions: [], action_inputs: [],
    fields: [
      { name: "appointment", shape: { type: "string" }, codec: { name: "rom.date", version: 1 } },
      { name: "amount", shape: { type: "string" }, codec: { name: "rom.decimal", version: 1 } },
      { name: "count", shape: { type: "u64" } },
      { name: "enabled", shape: { type: "bool" } },
      { name: "note", shape: { type: "optional", value: { type: "nullable", value: { type: "string" } } } },
    ],
  };
  const value: WireObject = {
    appointment: "2024-02-29", amount: "9007199254740993.000000000000000001",
    count: 9007199254740993n, enabled: false, note: "Keep this note",
  };
  let submitted = $state("none");
  let rejecting = $state(false);
  async function submit(input: ResourceFormSubmission) {
    if (rejecting) throw Error("Application rejected this patch.");
    submitted = stringifyWire(input);
  }
</script>

<main>
  <h1>Installed ROM form</h1>
  <label><input type="checkbox" bind:checked={rejecting} /> Reject application submission</label>
  <ResourceForm {descriptor} {value} mode="patch" direct {submit} />
  <output aria-label="Submitted form">{submitted}</output>
</main>
