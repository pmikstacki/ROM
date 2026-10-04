<script lang="ts">
  import { Switch } from "../components/ui/switch/index.js";
  import { Label } from "../components/ui/label/index.js";
  let {
    label,
    checked,
    onchange,
    disabled = false,
    showLabel = true,
  }: {
    label: string;
    checked: boolean;
    onchange: (checked: boolean) => void;
    disabled?: boolean;
    showLabel?: boolean;
  } = $props();
  const id = $props.id();
</script>

<div class="flex min-h-9 items-center justify-between gap-3">
  {#if showLabel}<Label for={id} class="text-sm font-normal"
      >{label.replaceAll("_", " ")}</Label
    >{/if}
  <Switch
    {id}
    aria-label={`${label} value`}
    {checked}
    onCheckedChange={onchange}
    {disabled}
  />
</div>
