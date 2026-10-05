<script lang="ts">
  import type { WireObject, WireValue } from "../client/types.ts";
  import { record } from "../client/validation.ts";
  import { stringifyWire } from "../client/codec.ts";
  import { workDefinition, workStatus } from "./work-presentation.ts";
  import { Button } from "../components/ui/button/index.js";
  import { Badge } from "../components/ui/badge/index.js";
  let { item }: { item: WireObject } = $props();
  let copied = $state(false),
    error = $state("");
  const version = $derived(record(item.version));
  function reference(value: WireValue): string {
    if (value === null) return "None";
    const key = record(value);
    return `${String(key.kind)} / ${String(key.id)}`;
  }
  async function copy() {
    copied = false;
    error = "";
    try {
      await navigator.clipboard.writeText(String(item.handle));
      copied = true;
    } catch {
      error = "Copy unavailable. Select the handle text to copy it.";
    }
  }
</script>

<section class="space-y-4" aria-label="Selected work details">
  <div class="space-y-1">
    <h2 class="break-words text-lg font-semibold">{workDefinition(item)}</h2>
    <p class="text-xs text-muted-foreground">
      {String(item.category)} · Definition version {String(
        record(item.definition).version,
      )}
    </p>
  </div>
  <Badge variant="outline">{workStatus(item.state)}</Badge>
  <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-2 text-sm">
    <dt class="text-muted-foreground">Attempts</dt>
    <dd>{String(item.attempts)}</dd>
    <dt class="text-muted-foreground">Due value</dt>
    <dd class="break-all">{String(item.due)}</dd>
    <dt class="text-muted-foreground">Delivery</dt>
    <dd>{item.delivery === null ? "Not recorded" : String(item.delivery)}</dd>
    <dt class="text-muted-foreground">Source</dt>
    <dd class="break-all">{reference(item.source)}</dd>
    <dt class="text-muted-foreground">Target</dt>
    <dd class="break-all">{reference(item.target)}</dd>
  </dl>
  <div class="space-y-2">
    <p class="text-xs text-muted-foreground">Work handle</p>
    <p class="break-all font-mono text-xs">{String(item.handle)}</p>
    <Button
      size="sm"
      variant="outline"
      aria-label="Copy work handle"
      onclick={() => void copy()}>{copied ? "Copied" : "Copy handle"}</Button
    >{#if error}<p role="alert" class="text-sm">{error}</p>{/if}
  </div>
  <details>
    <summary class="cursor-pointer text-sm text-muted-foreground"
      >Technical details</summary
    >
    <p class="mt-2 break-all text-xs">
      Generation {String(version.generation)} · Revision {String(
        version.revision,
      )}
    </p>
    <pre
      class="mt-2 overflow-x-auto rounded bg-muted p-2 text-xs">{stringifyWire(
        item,
      )}</pre>
  </details>
</section>
