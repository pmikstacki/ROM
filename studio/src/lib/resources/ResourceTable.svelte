<script lang="ts">
  import type { ResourceDescriptor, ProjectedView } from "../client/types.ts";
  import * as Table from "../components/ui/table/index.js";
  import { Button } from "../components/ui/button/index.js";
  import OpenIcon from "@lucide/svelte/icons/arrow-up-right";
  import ValueDisplay from "../renderers/ValueDisplay.svelte";
  import { resourceTitle } from "../presentation/resource-presentation.ts";
  let {
    descriptor,
    rows,
    onselect,
  }: {
    descriptor: ResourceDescriptor;
    rows: ProjectedView[];
    onselect: (row: ProjectedView, opener?: HTMLButtonElement) => void;
  } = $props();
</script>

<Table.Root class="text-sm"
  ><Table.Caption>{descriptor.kind} resources</Table.Caption><Table.Header
    ><Table.Row
      ><Table.Head scope="col">{descriptor.presentation?.title_field ? "Resource" : "ID"}</Table.Head><Table.Head scope="col"
        >Revision</Table.Head
      >{#each descriptor.fields as field}<Table.Head scope="col"
          >{descriptor.presentation?.fields?.[field.name]?.label ||
            field.name}</Table.Head
        >{/each}<Table.Head scope="col">Open</Table.Head></Table.Row
    ></Table.Header
  ><Table.Body
    >{#each rows as row (row.key.id)}<Table.Row
        ><Table.Cell class="max-w-48 truncate font-medium" title={row.key.id}
          >{descriptor.presentation?.title_field
            ? resourceTitle(descriptor, row)
            : row.key.id}</Table.Cell
        ><Table.Cell>{row.revision.toString()}</Table.Cell
        >{#each descriptor.fields as field}<Table.Cell
            ><ValueDisplay
              descriptor={field}
              value={row.value?.[field.name]}
              mode="cell"
            /></Table.Cell
          >{/each}<Table.Cell
          ><Button
            variant="outline"
            size="sm"
            class="max-lg:size-9"
            aria-label={`Open ${row.key.id}`}
            title={`Open ${row.key.id}`}
            onclick={(event) =>
              onselect(row, event.currentTarget as HTMLButtonElement)}
            ><OpenIcon /><span class="hidden lg:inline">Open {row.key.id}</span
            ></Button
          ></Table.Cell
        ></Table.Row
      >{/each}</Table.Body
  ></Table.Root
>
