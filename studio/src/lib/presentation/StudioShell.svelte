<script lang="ts">
  import type { Snippet } from "svelte";
  import type { ResourceDescriptor } from "../client/types.ts";
  import * as Sidebar from "../components/ui/sidebar/index.js";
  import { Separator } from "../components/ui/separator/index.js";
  import * as Breadcrumb from "../components/ui/breadcrumb/index.js";
  import StudioNavigation from "./StudioNavigation.svelte";
  let sidebarTrigger = $state<HTMLElement | null>(null);
  let {
    descriptors,
    kind,
    page,
    onpage,
    onkind,
    onsignout,
    children,
  }: {
    descriptors: ResourceDescriptor[];
    kind: string | null;
    page: "resources" | "work" | "attachments";
    onpage: (page: "resources" | "work" | "attachments") => void;
    onkind: (kind: string) => void;
    onsignout: () => void;
    children: Snippet;
  } = $props();
</script>

<Sidebar.Provider>
  <StudioNavigation
    {descriptors}
    {kind}
    {page}
    {onpage}
    {onkind}
    {onsignout}
    onMobileCloseAutoFocus={(event) => {
      event.preventDefault();
      sidebarTrigger?.focus();
    }}
  />
  <Sidebar.Inset class="min-w-0">
    <header class="flex h-14 shrink-0 items-center gap-3 border-b px-4 md:px-6">
      <Sidebar.Trigger
        bind:ref={sidebarTrigger}
        onclick={(event) => {
          // WebKit does not focus buttons on pointer activation. Establish the
          // return target before the mobile Sheet captures focus.
          if (event.currentTarget instanceof HTMLElement)
            event.currentTarget.focus();
        }}
      />
      <Separator orientation="vertical" class="h-4" />
      <Breadcrumb.Root class="min-w-0">
        <Breadcrumb.List class="flex-nowrap">
          <Breadcrumb.Item class="hidden sm:inline-flex"
            >Workspace</Breadcrumb.Item
          >
          <Breadcrumb.Separator class="hidden sm:block" />
          <Breadcrumb.Item class="min-w-0"
            ><Breadcrumb.Page class="truncate">
              {page === "resources"
                ? kind || "Resources"
                : page === "work"
                  ? "Work"
                  : "Attachments"}
            </Breadcrumb.Page></Breadcrumb.Item
          >
        </Breadcrumb.List>
      </Breadcrumb.Root>
      <span
        class="ml-auto hidden items-center gap-2 text-xs text-muted-foreground sm:flex"
        ><span class="size-1.5 rounded-full bg-emerald-600" aria-hidden="true"
        ></span>Connected</span
      >
    </header>
    <div class="studio-content flex min-w-0 flex-1 flex-col gap-6 p-4 md:p-6">
      {@render children()}
    </div>
  </Sidebar.Inset>
</Sidebar.Provider>
