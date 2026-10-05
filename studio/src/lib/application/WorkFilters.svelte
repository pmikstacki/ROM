<script lang="ts">
  import SelectAdapter from "../renderers/SelectAdapter.svelte";
  import { Input } from "../components/ui/input/index.js";
  import { Button } from "../components/ui/button/index.js";
  import { Badge } from "../components/ui/badge/index.js";
  import { workStates, workStatus } from "./work-presentation.ts";
  let {
    category = $bindable(""),
    statusFilter = $bindable(""),
    definition = $bindable(""),
    edited,
    disabled,
    canInspect,
    onapply,
  }: {
    category?: string;
    statusFilter?: string;
    definition?: string;
    edited: boolean;
    disabled: boolean;
    canInspect: boolean;
    onapply: () => void;
  } = $props();
</script>

<form
  class="space-y-4"
  onsubmit={(event) => {
    event.preventDefault();
    onapply();
  }}
>
  <div class="flex items-center justify-between gap-2">
    <h2 class="font-semibold">Work filters</h2>
    {#if edited}<Badge variant="outline">Pending edits</Badge>{/if}
  </div>
  <SelectAdapter
    label="Work category"
    value={category}
    {disabled}
    onchange={(value) => (category = value)}
    options={[
      { value: "", label: "All categories" },
      { value: "Reaction", label: "Reaction" },
      { value: "Notification", label: "Notification" },
    ]}
  />
  <SelectAdapter
    label="Work state"
    value={statusFilter}
    {disabled}
    onchange={(value) => (statusFilter = value)}
    options={[
      { value: "", label: "All states" },
      ...workStates.map((value) => ({ value, label: workStatus(value) })),
    ]}
  />
  <label class="block space-y-1 text-sm"
    >Work definition<Input bind:value={definition} {disabled} /></label
  >
  <p class="text-xs text-muted-foreground">Changes run only when applied.</p>
  <Button type="submit" disabled={disabled || !canInspect}
    >Apply work filters</Button
  >
</form>
