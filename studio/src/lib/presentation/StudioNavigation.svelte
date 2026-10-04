<script lang="ts">
  import * as Sidebar from "../components/ui/sidebar/index.js";
  import { Boxes, Database, Workflow, Paperclip, LogOut } from "@lucide/svelte";
  import type { ResourceDescriptor } from "../client/types.ts";

  let {
    descriptors,
    kind,
    page,
    onpage,
    onkind,
    onsignout,
    onMobileCloseAutoFocus,
  }: {
    descriptors: ResourceDescriptor[];
    kind: string | null;
    page: "resources" | "work" | "attachments";
    onpage: (page: "resources" | "work" | "attachments") => void;
    onkind: (kind: string) => void;
    onsignout: () => void;
    onMobileCloseAutoFocus: (event: Event) => void;
  } = $props();
  const sidebar = Sidebar.useSidebar();
  function tooltip(label: string) {
    return !sidebar.isMobile && sidebar.state === "collapsed"
      ? label
      : undefined;
  }
  function navigate(
    next: "resources" | "work" | "attachments",
    resource?: string,
  ) {
    onpage(next);
    if (resource) onkind(resource);
    sidebar.setOpenMobile(false);
  }
</script>

<Sidebar.Root variant="inset" collapsible="icon" {onMobileCloseAutoFocus}>
  <Sidebar.Header>
    <Sidebar.Menu
      ><Sidebar.MenuItem>
        <Sidebar.MenuButton
          size="lg"
          onclick={() => navigate("resources")}
          aria-label="ROM Studio home"
        >
          <span
            class="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary text-primary-foreground"
            ><Boxes class="size-4" /></span
          >
          <span class="flex flex-col text-left leading-tight"
            ><span class="font-semibold">ROM Studio</span><span
              class="text-xs text-muted-foreground">Resource workspace</span
            ></span
          >
        </Sidebar.MenuButton>
      </Sidebar.MenuItem></Sidebar.Menu
    >
  </Sidebar.Header>
  <Sidebar.Content>
    <Sidebar.Group>
      <Sidebar.GroupLabel>Workspace</Sidebar.GroupLabel>
      <Sidebar.Menu>
        <Sidebar.MenuItem
          ><Sidebar.MenuButton
            isActive={page === "resources"}
            aria-current={page === "resources" ? "true" : undefined}
            onclick={() => navigate("resources")}
            tooltipContent={tooltip("Resources")}
            ><Database /><span>Resources</span></Sidebar.MenuButton
          ></Sidebar.MenuItem
        >
        <Sidebar.MenuItem
          ><Sidebar.MenuButton
            isActive={page === "work"}
            aria-current={page === "work" ? "page" : undefined}
            onclick={() => navigate("work")}
            tooltipContent={tooltip("Work")}
            ><Workflow /><span>Work</span></Sidebar.MenuButton
          ></Sidebar.MenuItem
        >
        <Sidebar.MenuItem
          ><Sidebar.MenuButton
            isActive={page === "attachments"}
            aria-current={page === "attachments" ? "page" : undefined}
            onclick={() => navigate("attachments")}
            tooltipContent={tooltip("Attachments")}
            ><Paperclip /><span>Attachments</span></Sidebar.MenuButton
          ></Sidebar.MenuItem
        >
      </Sidebar.Menu>
    </Sidebar.Group>
    <Sidebar.Group>
      <Sidebar.GroupLabel>Resources</Sidebar.GroupLabel>
      <Sidebar.Menu aria-label="Resource types">
        {#each descriptors as descriptor (descriptor.kind)}
          <Sidebar.MenuItem
            ><Sidebar.MenuButton
              isActive={page === "resources" && kind === descriptor.kind}
              aria-current={page === "resources" && kind === descriptor.kind
                ? "page"
                : undefined}
              tooltipContent={tooltip(descriptor.kind)}
              onclick={() => navigate("resources", descriptor.kind)}
              ><Database /><span class="truncate">{descriptor.kind}</span
              ></Sidebar.MenuButton
            ></Sidebar.MenuItem
          >
        {/each}
      </Sidebar.Menu>
    </Sidebar.Group>
  </Sidebar.Content>
  <Sidebar.Footer>
    <Sidebar.Menu
      ><Sidebar.MenuItem
        ><Sidebar.MenuButton
          onclick={onsignout}
          tooltipContent={tooltip("Sign out")}
          ><LogOut /><span>Sign out</span></Sidebar.MenuButton
        ></Sidebar.MenuItem
      ></Sidebar.Menu
    >
  </Sidebar.Footer>
  <Sidebar.Rail />
</Sidebar.Root>
