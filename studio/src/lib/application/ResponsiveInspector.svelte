<script lang="ts">
  import type { Snippet } from "svelte";
  import { preserveEditor } from "./preserve-editor.ts";
  import { MediaQuery } from "svelte/reactivity";
  import { INSPECTOR_MEDIA_QUERY } from "./inspector-layout.ts";
  import * as Sheet from "../components/ui/sheet/index.js";
  let {
    open = $bindable(true),
    children,
    onCloseFocus = () => {},
    title = "Resource tools",
  }: {
    open?: boolean;
    children: Snippet;
    onCloseFocus?: () => void;
    title?: string;
  } = $props();
  const mobile = new MediaQuery(INSPECTOR_MEDIA_QUERY);
  let desktopTarget = $state<HTMLDivElement | null>(null);
  let mobileTarget = $state<HTMLDivElement | null>(null);
  const sheetOpen = $derived(mobile.current && open);
  // Bits Dialog's default force-mounted branch instantiates ScrollLock while
  // closed. Keep its inactive desktop host inert and lock only an open Sheet.
  $effect(() => {
    if (!sheetOpen) return;
    const body = document.body,
      previous = body.style.overflow;
    body.style.overflow = "hidden";
    return () => {
      if (body.style.overflow === "hidden") body.style.overflow = previous;
    };
  });
</script>

<Sheet.Root
  open={sheetOpen}
  onOpenChange={(next) => {
    if (mobile.current) open = next;
  }}
>
  <aside
    hidden={mobile.current || !open}
    class="min-w-0 rounded-lg border bg-card p-4 xl:sticky xl:top-4"
    aria-label="Resource tools"
  >
    <div bind:this={desktopTarget}></div>
  </aside>
  <Sheet.Content
    forceMount
    preventScroll={false}
    onCloseAutoFocus={(event) => {
      event.preventDefault();
      if (mobile.current) onCloseFocus();
    }}
    hidden={!sheetOpen}
    class="data-[side=right]:w-full data-[side=right]:sm:max-w-xl overflow-y-auto p-4"
    showCloseButton={true}
  >
    <Sheet.Header class="pr-8">
      <Sheet.Title>{title}</Sheet.Title>
      <Sheet.Description class="sr-only"
        >Filters and selected Resource details.</Sheet.Description
      >
    </Sheet.Header>
    <div bind:this={mobileTarget}></div>
  </Sheet.Content>
  <div
    use:preserveEditor={mobile.current ? mobileTarget : desktopTarget}
    class="min-w-0"
  >
    {@render children()}
  </div>
</Sheet.Root>
