<script lang="ts">
  import type { Snippet } from "svelte";
  import { ResponsiveDetails } from "rom-ui/ui/components";
  import { INSPECTOR_MEDIA_QUERY } from "./inspector-layout.ts";
  let {
    open = $bindable(true),
    children,
    onCloseFocus = () => {},
    title = "Resource tools",
    label = "Resource tools",
    description = "Filters and selected Resource details.",
    id,
  }: {
    open?: boolean;
    children: Snippet;
    onCloseFocus?: () => void;
    title?: string;
    label?: string;
    description?: string;
    id?: string;
  } = $props();
</script>

<ResponsiveDetails
  bind:open
  {children}
  {title}
  {label}
  {description}
  {id}
  closeLabel="Close"
  breakpoint={INSPECTOR_MEDIA_QUERY}
  fallbackFocus={onCloseFocus}
  desktopRole="complementary"
  showDesktopHeader={false}
/>
