<script lang="ts">
  import CheckboxAdapter from "../renderers/CheckboxAdapter.svelte";
  import { Input } from "../components/ui/input/index.js";
  import { Button } from "../components/ui/button/index.js";
  let {
    confirmation = $bindable(false),
    evidence = $bindable(""),
    hasSelection,
    canRetry,
    canReconcile,
    fresh,
    blocked,
    busy,
    pending,
    oncontrol,
    onretry,
  }: {
    confirmation?: boolean;
    evidence?: string;
    hasSelection: boolean;
    canRetry: boolean;
    canReconcile: boolean;
    fresh: boolean;
    blocked: boolean;
    busy: boolean;
    pending: boolean;
    oncontrol: (operation: "Retry" | "Reconcile") => void;
    onretry: () => void;
  } = $props();
</script>

{#if hasSelection}
  <section class="mt-5 space-y-3 border-t pt-4" aria-label="Work recovery">
    <h3 class="font-semibold">Recovery</h3>
    {#if !fresh}<p class="text-sm text-muted-foreground">
        Inspect the current version before another recovery action.
      </p>{/if}
    <CheckboxAdapter
      label="I confirm this recovery action"
      checked={confirmation}
      onchange={(next) => (confirmation = next)}
      disabled={blocked || !fresh}
    />
    {#if canRetry}<Button
        disabled={!confirmation || blocked || !fresh}
        onclick={() => oncontrol("Retry")}>Retry work</Button
      >{/if}
    {#if canReconcile}<label class="block space-y-1 text-sm"
        >Evidence reference<Input
          bind:value={evidence}
          disabled={blocked || !fresh}
        /></label
      >
      <Button
        disabled={!confirmation || blocked || !fresh}
        onclick={() => oncontrol("Reconcile")}>Reconcile work</Button
      >{/if}
    {#if !canRetry && !canReconcile}<p class="text-sm text-muted-foreground">
        Recovery is unavailable for this session.
      </p>{/if}
  </section>
{/if}
{#if pending}
  <div class="mt-4 space-y-3 rounded-lg border p-3">
    <p class="text-sm">
      The recovery result is not confirmed. Retry preserves the same request.
    </p>
    <Button disabled={busy} onclick={onretry}
      >Retry same recovery request</Button
    >
  </div>
{/if}
