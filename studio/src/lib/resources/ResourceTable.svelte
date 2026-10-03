<script lang="ts">
  import type { ResourceDescriptor, ProjectedView } from "../client/types.ts";
  import * as Table from "../components/ui/table/index.js";
  import { Button } from "../components/ui/button/index.js";
  import ValueDisplay from "../renderers/ValueDisplay.svelte";
  let {
    descriptor,
    rows,
    onselect,
  }: {
    descriptor: ResourceDescriptor;
    rows: ProjectedView[];
    onselect: (row: ProjectedView) => void;
  } = $props();
</script>

<Table.Root
  ><Table.Caption>{descriptor.kind} resources</Table.Caption><Table.Header
    ><Table.Row
      ><Table.Head scope="col">ID</Table.Head><Table.Head scope="col"
        >Revision</Table.Head
      >{#each descriptor.fields as field}<Table.Head scope="col"
          >{field.name}</Table.Head
        >{/each}<Table.Head scope="col">Open</Table.Head></Table.Row
    ></Table.Header
  ><Table.Body
    >{#each rows as row (row.key.id)}<Table.Row
        ><Table.Cell>{row.key.id}</Table.Cell><Table.Cell
          >{row.revision.toString()}</Table.Cell
        >{#each descriptor.fields as field}<Table.Cell
            ><ValueDisplay
              descriptor={field}
              value={row.value?.[field.name]}
              mode="cell"
            /></Table.Cell
          >{/each}<Table.Cell
          ><Button onclick={() => onselect(row)}>Open {row.key.id}</Button
          ></Table.Cell
        ></Table.Row
      >{/each}</Table.Body
  ></Table.Root
>
