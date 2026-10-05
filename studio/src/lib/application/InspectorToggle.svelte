<script lang="ts">
  import { Button } from "../components/ui/button/index.js";
  import * as Tooltip from "../components/ui/tooltip/index.js";
  import ChevronLeftIcon from "@lucide/svelte/icons/chevron-left";
  import ChevronRightIcon from "@lucide/svelte/icons/chevron-right";
  let {
    open,
    controls,
    onclick,
    ref = $bindable(null),
    label = "details panel",
  }: {
    open: boolean;
    controls: string;
    onclick: () => void;
    ref?: HTMLButtonElement | null;
    label?: string;
  } = $props();
  const name = $derived(`${open ? "Close" : "Open"} ${label}`);
</script>

<Tooltip.Provider>
  <Tooltip.Root>
    <Tooltip.Trigger {onclick}>
      {#snippet child({ props })}
        <Button
          {...props}
          bind:ref
          variant="ghost"
          size="icon"
          aria-label={name}
          aria-expanded={open}
          aria-controls={controls}
        >
          {#if open}<ChevronRightIcon />{:else}<ChevronLeftIcon />{/if}
        </Button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content>{name}</Tooltip.Content>
  </Tooltip.Root>
</Tooltip.Provider>
