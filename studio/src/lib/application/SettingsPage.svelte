<script lang="ts">
  import type {
    ApplicationController,
    ApplicationState,
  } from "./controller.ts";
  import {
    resourceLabel,
    settingsSections,
  } from "../presentation/resource-presentation.ts";
  import ResourcePage from "./ResourcePage.svelte";
  import { Button } from "../components/ui/button/index.js";
  let {
    controller,
    snapshot,
    navigationBlocked = false,
  }: {
    controller: ApplicationController;
    snapshot: ApplicationState;
    navigationBlocked?: boolean;
  } = $props();
  const sections = $derived(settingsSections(snapshot.descriptors));
  const descriptor = $derived(
    snapshot.descriptors.find(
      (item) => item.kind === snapshot.kind && item.presentation?.settings,
    ),
  );
</script>

<div>
  <h1 class="text-xl font-semibold tracking-tight">Settings</h1>
  <p class="text-sm text-muted-foreground">
    Manage the settings available to this session.
  </p>
</div>
{#if sections.length}
  <nav
    aria-label="Settings groups"
    class="flex flex-wrap gap-4 rounded-lg border bg-card p-3"
  >
    {#each sections as section (section.group)}
      <div class="space-y-2">
        <h2 class="text-xs font-semibold text-muted-foreground">
          {section.label}
        </h2>
        <div class="flex flex-wrap gap-1">
          {#each section.resources as resource (resource.kind)}
            <Button
              variant={snapshot.kind === resource.kind ? "secondary" : "ghost"}
              size="sm"
              aria-current={snapshot.kind === resource.kind
                ? "page"
                : undefined}
              disabled={navigationBlocked}
              onclick={() => {
                if (!navigationBlocked)
                  void controller.selectKind(resource.kind);
              }}>{resourceLabel(resource)}</Button
            >
          {/each}
        </div>
      </div>
    {/each}
  </nav>
  {#if descriptor}
    {#key descriptor.kind}<ResourcePage
        {controller}
        {snapshot}
        {descriptor}
      />{/key}
  {:else}
    <p class="text-sm text-muted-foreground">Choose a Settings section.</p>
  {/if}
{:else}
  <div class="space-y-2 rounded-lg border bg-card p-4">
    <h2 class="font-semibold">No Settings available</h2>
    <p class="text-sm text-muted-foreground">
      This session has no disclosed Settings Resources.
    </p>
  </div>
{/if}
