<script lang="ts">
  import { onDestroy, onMount, setContext, untrack } from "svelte";
  import { createClient } from "./lib/client/client.ts";
  import type { RomClient } from "./lib/client/types.ts";
  import { createApplication } from "./lib/application/controller.ts";
  import LoginPage from "./lib/application/LoginPage.svelte";
  import { REFERENCE_LOOKUP } from "./lib/renderers/reference-lookup.ts";
  import ResourcePage from "./lib/application/ResourcePage.svelte";
  import AttachmentPage from "./lib/application/AttachmentPage.svelte";
  import WorkPage from "./lib/application/WorkPage.svelte";
  import SettingsPage from "./lib/application/SettingsPage.svelte";
  import {
    navigationBlocked,
    type StudioPage,
  } from "./lib/presentation/navigation.ts";
  import { settingsSections } from "./lib/presentation/resource-presentation.ts";
  import StudioShell from "./lib/presentation/StudioShell.svelte";
  import * as Card from "./lib/components/ui/card/index.js";
  import * as Alert from "./lib/components/ui/alert/index.js";
  import { Button } from "./lib/components/ui/button/index.js";
  import {
    createBrowserAuth,
    SessionExpiredError,
  } from "./lib/application/auth.ts";
  import type { ProviderChoice } from "./lib/application/auth.ts";
  const base = import.meta.env.BASE_URL;
  const auth = createBrowserAuth(base);
  let {
    client = createClient({ base: `${base}api`, csrf: auth.csrf }),
  }: { client?: RomClient } = $props();
  let providers = $state<ProviderChoice[]>([]),
    authError = $state(""),
    checking = $state(false),
    primaryProvider = $state<string | null>(null),
    sessionExpired = $state(false);
  let sessionGeneration: string | undefined;
  let destroyed = false;
  async function showProviders() {
    const choices = await auth.providers();
    if (destroyed) return;
    providers = choices.providers;
    primaryProvider = choices.primary ?? null;
    if (choices.primary) {
      try {
        const flag = "rom-primary-redirect";
        if (!sessionStorage.getItem(flag)) {
          sessionStorage.setItem(flag, "1");
          location.assign(auth.loginUrl(choices.primary));
        }
      } catch {
        /* Provider buttons remain available when session storage is blocked. */
      }
    }
  }
  async function sessionCheck(connect = false) {
    if (checking) return;
    checking = true;
    try {
      const session = await auth.refresh();
      if (destroyed) return;
      if (!session.authenticated) {
        if (sessionGeneration !== undefined) sessionExpired = true;
        controller.disconnect();
        sessionGeneration = undefined;
        await showProviders();
      } else {
        try {
          sessionStorage.removeItem("rom-primary-redirect");
        } catch {}
        if (
          sessionGeneration !== undefined &&
          sessionGeneration !== session.generation
        )
          controller.disconnect();
        sessionExpired = false;
        primaryProvider = null;
        sessionGeneration = session.generation;
        providers = [];
        if (connect || controller.state.phase === "disconnected")
          await controller.connect();
      }
      authError = "";
    } catch (problem) {
      controller.disconnect();
      if (problem instanceof SessionExpiredError) {
        sessionExpired = true;
        sessionGeneration = undefined;
        try {
          await showProviders();
        } catch {
          providers = [];
          primaryProvider = null;
        }
      }
      authError =
        problem instanceof Error ? problem.message : "Session unavailable.";
    } finally {
      checking = false;
    }
  }
  async function signOut() {
    sessionExpired = false;
    controller.disconnect();
    sessionGeneration = undefined;
    try {
      await auth.logout();
      await sessionCheck();
    } catch (problem) {
      authError =
        problem instanceof Error
          ? problem.message
          : "Logout was not confirmed.";
    }
  }
  onMount(() => {
    void sessionCheck(true);
    const timer = setInterval(() => void sessionCheck(), 15000);
    return () => clearInterval(timer);
  });
  const controller = createApplication(
    untrack(() => client),
    undefined,
    async () => {
      const session = await auth.refresh();
      return session.authenticated && session.generation === sessionGeneration;
    },
  );
  let snapshot = $state.raw(controller.state),
    page = $state<StudioPage>("resources"),
    workNavigationBlocked = $state(false),
    attachmentNavigationBlocked = $state(false);
  setContext(REFERENCE_LOOKUP, {
    descriptor: (kind: string) =>
      snapshot.phase === "ready"
        ? snapshot.descriptors.find((item) => item.kind === kind)
        : undefined,
    lookup: (kind: string, search: string, signal: AbortSignal) =>
      controller.lookupResources(kind, search, signal),
  });
  const blockedNavigation = $derived(
    navigationBlocked(
      snapshot.pending?.state,
      workNavigationBlocked || attachmentNavigationBlocked,
    ),
  );
  const unsubscribe = controller.subscribe((next) => {
    snapshot = next;
    if (next.phase !== "ready") {
      workNavigationBlocked = false;
      attachmentNavigationBlocked = false;
    }
  });
  function navigate(next: StudioPage) {
    if (blockedNavigation) return;
    page = next;
    if (next === "settings" && !descriptor?.presentation?.settings) {
      const first = settingsSections(snapshot.descriptors)[0]?.resources[0];
      if (first) void controller.selectKind(first.kind);
    }
  }
  onDestroy(() => {
    destroyed = true;
    unsubscribe();
    controller.disconnect();
  });
  let descriptor = $derived(
    snapshot.descriptors.find((item) => item.kind === snapshot.kind),
  );
</script>

{#snippet noticesContent()}
  {#if authError}<Alert.Root variant="destructive"
      ><Alert.Description>{authError}</Alert.Description></Alert.Root
    >{/if}
  {#if snapshot.error}<Alert.Root variant="destructive"
      ><Alert.Description>{snapshot.error}</Alert.Description></Alert.Root
    >{/if}
  {#if snapshot.pending}
    <Alert.Root aria-label="Mutation outcome">
      <Alert.Title>Mutation outcome</Alert.Title>
      <Alert.Description>
        <p>
          {snapshot.pending.state} · {snapshot.pending.request.kind}/{snapshot
            .pending.request.id}
        </p>
        {#if snapshot.pending.state === "unknown"}
          <p class="mt-2">
            The result is unknown. Retry uses the same operation and idempotency
            key.
          </p>
          <Button
            class="mt-3"
            size="sm"
            variant="outline"
            disabled={snapshot.busy}
            onclick={() => void controller.retry().catch(() => {})}
            >Retry same mutation</Button
          >
        {/if}
      </Alert.Description>
    </Alert.Root>
  {/if}
{/snippet}

{#if snapshot.phase === "disconnected" || snapshot.phase === "error"}
  <LoginPage
    {providers}
    {checking}
    primary={primaryProvider}
    {sessionExpired}
    loginUrl={(id) => auth.loginUrl(id)}
    onCheck={() => void sessionCheck(true)}
  >
    {#snippet notices()}{@render noticesContent()}{/snippet}
  </LoginPage>
{:else if snapshot.phase === "connecting"}
  <main class="flex min-h-svh items-center justify-center">
    <p role="status" class="text-sm text-muted-foreground">
      Connecting to ROM…
    </p>
  </main>
{:else}
  <StudioShell
    descriptors={snapshot.descriptors}
    kind={snapshot.kind}
    {page}
    onpage={navigate}
    onkind={(kind) => {
      if (!blockedNavigation) void controller.selectKind(kind);
    }}
    navigationBlocked={blockedNavigation}
    onsignout={() => void signOut()}
  >
    {@render noticesContent()}
    {#if page === "attachments"}<AttachmentPage
        {client}
        descriptors={snapshot.descriptors}
        onNavigationBlockChange={(blocked) =>
          (attachmentNavigationBlocked = blocked)}
      />
    {:else if page === "work"}<WorkPage
        {snapshot}
        {controller}
        onNavigationBlockChange={(blocked) => (workNavigationBlocked = blocked)}
      />
    {:else if page === "settings"}<SettingsPage
        {snapshot}
        {controller}
        navigationBlocked={blockedNavigation}
      />
    {:else if descriptor}{#key descriptor.kind}<ResourcePage
          {snapshot}
          {controller}
          {descriptor}
        />{/key}
    {:else}<Card.Root
        ><Card.Header
          ><Card.Title>No Resources available</Card.Title><Card.Description
            >This session has no authorized Resource descriptors.</Card.Description
          ></Card.Header
        ></Card.Root
      >{/if}
  </StudioShell>
{/if}
