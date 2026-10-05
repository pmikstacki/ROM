<script lang="ts">
  import type { Snippet } from "svelte";
  import type { ProviderChoice } from "./auth.ts";
  import * as Card from "../components/ui/card/index.js";
  import { Button } from "../components/ui/button/index.js";
  import { Boxes, LogIn, RefreshCw } from "@lucide/svelte";
  let {
    providers,
    checking,
    loginUrl,
    onCheck,
    primary = null,
    sessionExpired = false,
    notices,
  }: {
    providers: ProviderChoice[];
    checking: boolean;
    loginUrl: (id: string) => string;
    onCheck: () => void;
    primary?: string | null;
    sessionExpired?: boolean;
    notices?: Snippet;
  } = $props();
</script>

<main class="flex min-h-svh items-center justify-center bg-muted/40 p-4 sm:p-6">
  <div class="w-full min-w-0 max-w-sm space-y-5">
    <div class="flex items-center justify-center gap-2.5">
      <span
        class="flex size-9 items-center justify-center rounded-lg bg-primary text-primary-foreground"
        ><Boxes class="size-5" aria-hidden="true" /></span
      >
      <h1 class="text-lg font-semibold tracking-tight">ROM Studio</h1>
    </div>
    <Card.Root>
      <Card.Header>
        <Card.Title><h2>Connect to ROM</h2></Card.Title>
        <Card.Description>
          {#if sessionExpired}Your session ended. Sign in again to continue.
          {:else}Sign in to your workspace with an available provider.{/if}
        </Card.Description>
      </Card.Header>
      <Card.Content class="space-y-3">
        {#if checking}<p role="status" class="text-sm text-muted-foreground">
            Checking your session…
          </p>{/if}
        {#each providers as provider}
          <Button
            class="h-auto min-h-9 w-full whitespace-normal break-words py-2 text-left"
            href={loginUrl(provider.id)}
            variant={provider.id === primary ? "default" : "outline"}
          >
            <LogIn class="shrink-0" aria-hidden="true" /><span class="min-w-0"
              >Sign in with {provider.label}</span
            >
          </Button>
          {#if provider.id === primary}<p class="text-xs text-muted-foreground">
              Preferred sign-in provider
            </p>{/if}
        {/each}
        {#if !checking && providers.length === 0}<p
            role="status"
            class="text-sm text-muted-foreground"
          >
            No sign-in providers are available. Check your connection or contact
            your workspace administrator.
          </p>{/if}
        <Button
          class="w-full"
          variant="outline"
          disabled={checking}
          onclick={onCheck}
        >
          <RefreshCw aria-hidden="true" /><span
            >{checking ? "Checking connection…" : "Check connection"}</span
          >
        </Button>
      </Card.Content>
    </Card.Root>
    {#if notices}{@render notices()}{/if}
    <p class="text-center text-xs text-muted-foreground">
      Your access follows your workspace permissions.
    </p>
  </div>
</main>
