<script lang="ts">
  import * as Popover from "../components/ui/popover/index.js";
  import { Input } from "../components/ui/input/index.js";
  import { Button } from "../components/ui/button/index.js";
  import CheckboxAdapter from "./CheckboxAdapter.svelte";
  import { enumLabel } from "../client/enum-labels.ts";
  let {
    members,
    labels,
    label,
    remaining,
    onappend,
  }: {
    members: string[];
    labels?: Record<string, string>;
    label: string;
    remaining: number;
    onappend: (members: string[]) => void;
  } = $props();
  let open = $state(false),
    search = $state(""),
    selected = $state<string[]>([]);
  $effect(() => {
    if (!open) {
      selected = [];
      search = "";
    }
  });
  const visible = $derived(
    members.filter((member) =>
      `${enumLabel(member, labels)} ${member}`
        .toLowerCase()
        .includes(search.toLowerCase()),
    ),
  );
  function toggle(member: string, checked: boolean) {
    selected = checked
      ? [...selected, member]
      : selected.filter((value) => value !== member);
  }
  function append() {
    if (!selected.length || selected.length > remaining) return;
    onappend(members.filter((member) => selected.includes(member)));
    selected = [];
    search = "";
    open = false;
  }
</script>

<Popover.Root bind:open>
  <Popover.Trigger
    class="inline-flex h-8 items-center rounded-md border px-3 text-sm font-medium hover:bg-accent"
    disabled={remaining === 0}
    aria-label={`Choose ${label} items`}>Choose items</Popover.Trigger
  >
  <Popover.Content
    class="w-80 max-w-[calc(100vw-2rem)] space-y-3"
    aria-label={`Choose ${label} items`}
  >
    <Input
      type="search"
      maxlength={256}
      aria-label={`Search ${label} choices`}
      placeholder="Search choices"
      bind:value={search}
    />
    <div class="max-h-60 overflow-auto">
      {#each visible as member (member)}
        <CheckboxAdapter
          label={`Select ${enumLabel(member, labels)} (${member}) for ${label}`}
          text={enumLabel(member, labels) === member
            ? member
            : `${enumLabel(member, labels)} (${member})`}
          checked={selected.includes(member)}
          disabled={!selected.includes(member) && selected.length >= remaining}
          onchange={(checked) => toggle(member, checked)}
        />
      {/each}
      {#if !visible.length}<p class="text-sm text-muted-foreground">
          No matching choices.
        </p>{/if}
    </div>
    <p class="text-xs text-muted-foreground">
      Selected choices append in schema order. Existing items keep their order.
    </p>
    <Button
      type="button"
      size="sm"
      disabled={!selected.length || selected.length > remaining}
      onclick={append}
      aria-label={`Append selected ${label} items`}
      >Append {selected.length} items</Button
    >
  </Popover.Content>
</Popover.Root>
