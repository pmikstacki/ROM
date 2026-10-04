<script lang="ts">
  import { Checkbox } from "../components/ui/checkbox/index.js";
  import { Label } from "../components/ui/label/index.js";
  let {
    label,
    text = label,
    checked,
    onchange,
    disabled = false,
  }: {
    label: string;
    text?: string;
    checked: boolean;
    onchange: (checked: boolean) => void;
    disabled?: boolean;
  } = $props();
  const id = $props.id();
</script>

<div class="flex items-center gap-2 py-1">
  <Checkbox
    {id}
    {checked}
    onCheckedChange={onchange}
    {disabled}
    aria-label={label}
  />
  {#if text}<Label for={id} class="text-sm font-normal">{text}</Label>{/if}
</div>
