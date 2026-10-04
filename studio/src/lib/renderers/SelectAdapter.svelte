<script lang="ts">
  import * as Select from "../components/ui/select/index.js";
  let {
    label,
    value,
    options,
    onchange,
    disabled = false,
    invalid = false,
  }: {
    label: string;
    value: string;
    options: { value: string; label: string }[];
    onchange: (value: string) => void;
    disabled?: boolean;
    invalid?: boolean;
  } = $props();
  // Index tokens keep empty and arbitrary Resource strings valid Select values.
  const selected = $derived(
    options.findIndex((option) => option.value === value),
  );
  const token = $derived(selected < 0 ? "" : String(selected));
  function choose(next: string) {
    if (disabled) return;
    const index = Number(next);
    if (next !== "" && Number.isInteger(index) && options[index])
      onchange(options[index].value);
  }
</script>

<Select.Root type="single" value={token} onValueChange={choose} {disabled}>
  <Select.Trigger
    class="w-full min-w-36"
    aria-label={label}
    aria-invalid={invalid}
  >
    <span data-slot="select-value"
      >{options[selected]?.label ?? "Choose a value"}</span
    >
  </Select.Trigger>
  <Select.Content>
    {#each options as option, index (option.value)}
      <Select.Item
        value={String(index)}
        label={option.label}
        {disabled}
        data-rom-select-value={option.value}
      />
    {/each}
  </Select.Content>
</Select.Root>
