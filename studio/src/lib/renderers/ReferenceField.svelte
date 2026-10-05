<script lang="ts">
  import { getContext } from "svelte";
  import type { WireValue, ResourceDescriptor } from "../client/types.ts";
  import { normalizeValue } from "../client/codec.ts";
  import { Input } from "../components/ui/input/index.js";
  import * as Popover from "../components/ui/popover/index.js";
  import * as Command from "../components/ui/command/index.js";
  import SearchIcon from "@lucide/svelte/icons/search";
  import {
    REFERENCE_LOOKUP,
    REFERENCE_SEARCH_BYTES,
    type ReferenceLookup,
    type ReferenceLookupResult,
  } from "./reference-lookup.ts";
  let {
    kind,
    value,
    label,
    path = label,
    onchange,
    onerror = () => {},
    readonly = false,
    showLabel = true,
  }: {
    kind: string;
    value: WireValue;
    label: string;
    path?: string;
    onchange: (value: WireValue) => void;
    onerror?: (message: string) => void;
    readonly?: boolean;
    showLabel?: boolean;
  } = $props();
  const lookup = getContext<ReferenceLookup | undefined>(REFERENCE_LOOKUP);
  const descriptor = $derived(lookup?.descriptor(kind));
  let open = $state(false),
    search = $state(""),
    loading = $state(false),
    invalid = $state("");
  let result = $state.raw<ReferenceLookupResult | undefined>();
  let chosen = $state.raw<
    { id: string; title: string; descriptor: ResourceDescriptor } | undefined
  >();
  let requestCount = 0;
  const chosenTitle = $derived(
    chosen?.id === value && chosen.descriptor === descriptor
      ? chosen.title
      : "",
  );
  function input(text: string) {
    if (readonly) return;
    if (chosen?.id !== text) chosen = undefined;
    try {
      if (text.length > 65536) throw Error("Field input limit reached.");
      const next = normalizeValue(
        { type: "reference", value: { kind } },
        text,
        path,
      );
      invalid = "";
      onerror("");
      onchange(next);
    } catch (problem) {
      invalid =
        problem instanceof Error ? problem.message : "Invalid reference ID.";
      onerror(invalid);
    }
  }
  function choose(id: string, title: string) {
    if (!descriptor || readonly) return;
    input(id);
    chosen = { id, title, descriptor };
    open = false;
  }
  $effect(() => {
    const currentDescriptor = descriptor,
      term = search;
    if (!open || readonly || !lookup || !currentDescriptor) {
      if (readonly || !lookup || !currentDescriptor) open = false;
      if (!lookup || !currentDescriptor) chosen = undefined;
      result = undefined;
      loading = false;
      requestCount = 0;
      return;
    }
    result = undefined;
    chosen = undefined;
    loading = true;
    const controller = new AbortController();
    const timer = setTimeout(
      async () => {
        if (
          new TextEncoder().encode(term).byteLength > REFERENCE_SEARCH_BYTES
        ) {
          result = {
            status: "error",
            message: "Reference search is too long.",
          };
          loading = false;
          return;
        }
        if (requestCount >= 8) {
          result = {
            status: "error",
            message:
              "Reference search limit reached. Enter an exact Resource ID.",
          };
          loading = false;
          return;
        }
        requestCount++;
        try {
          const next = await lookup.lookup(kind, term, controller.signal);
          if (
            !controller.signal.aborted &&
            lookup.descriptor(kind) === currentDescriptor
          )
            result = next;
        } catch {
          if (!controller.signal.aborted)
            result = {
              status: "error",
              message: "Reference lookup failed. Enter an exact Resource ID.",
            };
        } finally {
          if (!controller.signal.aborted) loading = false;
        }
      },
      term ? 250 : 0,
    );
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  });
</script>

<div class="min-w-0 space-y-1.5">
  {#if showLabel}<span class="text-sm">{label}</span>{/if}
  <div class="flex min-w-0 items-center gap-2">
    <Input
      type="search"
      aria-label={`${label} value`}
      value={typeof value === "string" ? value : ""}
      placeholder={`Resource ID in ${kind}`}
      disabled={readonly}
      aria-invalid={!!invalid}
      class="min-w-0 flex-1"
      oninput={(event) => input(event.currentTarget.value)}
    />
    {#if lookup && descriptor}
      <Popover.Root bind:open>
        <Popover.Trigger
          type="button"
          disabled={readonly}
          aria-label={`Choose ${label} reference`}
          class="inline-flex size-9 shrink-0 items-center justify-center rounded-md border border-input hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
          ><SearchIcon class="size-4" /></Popover.Trigger
        >
        <Popover.Content
          align="end"
          class="w-[min(24rem,calc(100vw-2rem))] gap-2 p-2"
        >
          <Command.Root shouldFilter={false}>
            <Command.Input
              bind:value={search}
              aria-label={`Search ${label} candidates`}
              placeholder="Search these candidates"
              maxlength={256}
            />
            <Command.List class="max-h-64 overflow-y-auto">
              {#if loading}<p role="status" class="p-3 text-sm">
                  Loading authorized candidates…
                </p>
              {:else if result?.status === "ready"}
                {#if result.candidates.length === 0}<p
                    role="status"
                    class="p-3 text-sm"
                  >
                    No matches in these authorized candidates.
                  </p>{/if}
                {#each result.candidates as candidate, index (candidate.id)}
                  <Command.Item
                    value={String(index)}
                    onSelect={() => choose(candidate.id, candidate.title)}
                    ><span class="min-w-0 space-y-0.5"
                      ><span class="block break-words">{candidate.title}</span
                      ><span
                        class="block break-all text-xs text-muted-foreground"
                        >{candidate.id}</span
                      ></span
                    ></Command.Item
                  >
                {/each}
              {:else if result}<p role="status" class="p-3 text-sm">
                  {result.message}
                </p>{/if}
            </Command.List>
          </Command.Root>
          <p class="px-2 text-xs text-muted-foreground">
            Search checks up to 20 authorized candidates.{#if result?.status === "ready" && result.limited}
              More Resources may exist.{/if} Enter an exact ID for another Resource.
          </p>
        </Popover.Content>
      </Popover.Root>
    {/if}
  </div>
  {#if chosenTitle}<p class="break-words text-sm">{chosenTitle}</p>{/if}
  {#if showLabel}<p class="text-xs text-muted-foreground">
      Resource ID in {kind}.{#if !descriptor}
        Candidates are unavailable. Enter an exact ID.{/if}
    </p>{/if}
  {#if invalid}<p role="alert">{invalid}</p>{/if}
</div>
