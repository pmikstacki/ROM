<script lang="ts">
  import * as Select from "../components/ui/select/index.js";
  import MoreHorizontalIcon from "@lucide/svelte/icons/more-horizontal";
  let {
    label,
    value,
    options,
    onchange,
    disabled = false,
    invalid = false,
    compact = false,
  }: {
    label: string;
    value: string;
    options: { value: string; label: string }[];
    onchange: (value: string) => void;
    disabled?: boolean;
    invalid?: boolean;
    compact?: boolean;
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
    class={compact
      ? "relative size-9 min-w-0 justify-center border-0 bg-transparent p-0 shadow-none [&>svg:last-child]:hidden"
      : "w-full min-w-36"}
    aria-label={label}
    aria-invalid={invalid}
    title={compact ? options[selected]?.label : undefined}
  >
    <span data-slot="select-value"
      >{#if compact}<MoreHorizontalIcon class="size-4" />{:else}{options[selected]
          ?.label ?? "Choose a value"}{/if}</span
    >
    {#if compact && value !== "omit"}<span
        aria-hidden="true"
        class="absolute right-0 top-0 size-1.5 rounded-full bg-primary"
      ></span>{/if}
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
