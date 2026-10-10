<script lang="ts">
  import { onDestroy, onMount, setContext, untrack } from "svelte";
  import { createClient } from "./lib/client/client.ts";
  import type { RomClient } from "./lib/client/types.ts";
  import { createApplication } from "./lib/application/controller.ts";
  import { createAppSession } from "./lib/application/app-session.ts";
  import type { StudioAuthProfile } from "./lib/application/session-types.ts";
  import type { SessionLifecycleState } from "./lib/auth/types.ts";
  import {
    formFrameDefinition,
    readFormFrames,
  } from "./lib/resources/form-draft.ts";
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
    client: suppliedClient,
    authProfile,
  }: { client?: RomClient; authProfile?: StudioAuthProfile } = $props();
  const managed = untrack(() =>
    authProfile
      ? createAppSession({ base, profile: authProfile, client: suppliedClient })
      : null,
  );
  const client =
    managed?.client ??
    untrack(
      () =>
        suppliedClient ?? createClient({ base: `${base}api`, csrf: auth.csrf }),
    );
  let sessionState = $state.raw<SessionLifecycleState | null>(
    managed?.lifecycle.state ?? null,
  );
  let authView = 0;
  let restoreEpoch = $state(0);
  const loginUrl = (id: string) =>
    managed ? managed.loginUrl(id) : auth.loginUrl(id);
  let providers = $state<ProviderChoice[]>([]),
    authError = $state(""),
    checking = $state(false),
    primaryProvider = $state<string | null>(null),
    sessionExpired = $state(false);
  let sessionGeneration: string | undefined;
  let destroyed = false;
  async function showProviders() {
    const ticket = authView;
    const choices = await (managed ? managed.providers() : auth.providers());
    if (destroyed || ticket !== authView) return;
    providers = choices.providers;
    primaryProvider = choices.primary ?? null;
    if (choices.primary) {
      try {
        const flag = "rom-primary-redirect";
        if (!sessionStorage.getItem(flag)) {
          sessionStorage.setItem(flag, "1");
          location.assign(loginUrl(choices.primary));
        }
      } catch {
        /* Provider buttons remain available when session storage is blocked. */
      }
    }
  }
  async function sessionCheck(connect = false) {
    if (checking) return;
    checking = true;
    const ticket = ++authView;
    try {
      if (managed) {
        const result = await managed.refresh();
        if (destroyed || ticket !== authView) return;
        if (result.status === "authenticated") {
          sessionExpired = false;
          providers = [];
          primaryProvider = null;
          authError = "";
          try {
            sessionStorage.removeItem("rom-primary-redirect");
          } catch {}
        } else if (result.status === "transient") {
          authError =
            "Session temporarily unavailable. Your draft and selected context are retained.";
        } else {
          sessionExpired = true;
          await showProviders();
          if (destroyed || ticket !== authView) return;
          authError = "";
        }
        return;
      }
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
      if (managed) {
        if (!destroyed && ticket === authView)
          authError = "Session unavailable.";
        return;
      }
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
      if (!destroyed && ticket === authView) checking = false;
    }
  }
  async function signOut() {
    if (managed) {
      ++authView;
      checking = false;
      providers = [];
      primaryProvider = null;
      sessionExpired = false;
      try {
        await managed.logout();
        if (!destroyed) await sessionCheck();
      } catch {
        if (!destroyed) authError = "Logout was not confirmed.";
      }
      return;
    }
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
  async function restoreDraft() {
    const ticket = authView;
    const owner = ownerKey;
    const target = snapshot.selected?.key;
    if (!managed || !target) return;
    try {
      await controller.restoreSelectedIntent();
      if (
        destroyed ||
        ticket !== authView ||
        owner !== ownerKey ||
        target.kind !== snapshot.selected?.key.kind ||
        target.id !== snapshot.selected.key.id
      )
        return;
      const restored = snapshot.editor?.snapshot;
      if (restored) {
        const descriptor = snapshot.descriptors.find(
          (item) => item.kind === target.kind,
        );
        try {
          if (!descriptor) throw Error("Resource definition unavailable.");
          readFormFrames(restored, formFrameDefinition(descriptor), {
            maxBytes: authProfile!.recovery.maxBytes,
          });
        } catch {
          controller.createDraftWriter(target.id).refuse();
          return;
        }
      }
      restoreEpoch++;
    } catch {}
  }
  onMount(() => {
    void sessionCheck(true);
    const timer = setInterval(() => void sessionCheck(), 15000);
    return () => clearInterval(timer);
  });
  const controller =
    managed?.controller ??
    createApplication(
      untrack(() => client),
      undefined,
      async () => {
        const session = await auth.refresh();
        return (
          session.authenticated && session.generation === sessionGeneration
        );
      },
    );
  const unsubscribeSession = managed?.lifecycle.subscribe((next) => {
    const previous = sessionState?.identity;
    sessionState = next;
    if (previous && !next.identity) {
      ++authView;
      checking = false;
      sessionExpired = true;
      authError = "";
      providers = [];
      primaryProvider = null;
      if (!destroyed && next.status !== "disposed")
        void showProviders().catch(() => {});
    }
  });
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
      workNavigationBlocked ||
        attachmentNavigationBlocked ||
        !!snapshot.recovery?.state.hasUnresolvedIntent ||
        snapshot.recovery?.state.phase === "storage_error" ||
        snapshot.editor?.status === "writing" ||
        snapshot.editor?.status === "error" ||
        !!snapshot.creation?.busy ||
        !!snapshot.creation?.recovery?.state.hasUnresolvedIntent ||
        snapshot.creation?.recovery?.state.phase === "storage_error" ||
        snapshot.creation?.editor.status === "writing" ||
        snapshot.creation?.editor.status === "error",
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
    ++authView;
    unsubscribe();
    unsubscribeSession?.();
    if (managed) managed.destroy();
    else controller.disconnect();
  });
  let descriptor = $derived(
    snapshot.descriptors.find((item) => item.kind === snapshot.kind),
  );
  const ownerKey = $derived(
    sessionState?.identity
      ? JSON.stringify(sessionState.identity.principal)
      : "legacy",
  );
</script>

{#snippet noticesContent()}
  {#if managed && snapshot.session?.status === "active"}
    <Button
      size="sm"
      variant="outline"
      disabled={checking}
      onclick={() => void sessionCheck()}>Check session</Button
    >
  {/if}
  {#if authError}<Alert.Root variant="destructive"
      ><Alert.Description>{authError}</Alert.Description></Alert.Root
    >{/if}
  {#if snapshot.error}<Alert.Root variant="destructive"
      ><Alert.Description>{snapshot.error}</Alert.Description></Alert.Root
    >{/if}
  {#if managed && snapshot.session?.status !== "active" && snapshot.phase === "ready"}
    <Alert.Root aria-label="Session recovery"
      ><Alert.Title>Session recovery</Alert.Title>
      <Alert.Description
        ><p>
          {snapshot.session?.status === "denied"
            ? "Current authorization no longer permits this view."
            : "Context is stale. Check your session before saving."}
        </p>
        <Button
          class="mt-3"
          size="sm"
          variant="outline"
          disabled={checking}
          onclick={() => void sessionCheck()}>Check session</Button
        >
      </Alert.Description></Alert.Root
    >
  {/if}
  {#if snapshot.recovery?.state.hasUnresolvedIntent || snapshot.recovery?.state.phase === "storage_error"}
    <Alert.Root aria-label="Mutation recovery"
      ><Alert.Title>Mutation recovery</Alert.Title><Alert.Description>
        <p>
          {snapshot.recovery.state.phase}. Commit knowledge: {snapshot.recovery
            .state.commitKnowledge}.
        </p>
        <p>
          The accepted command is retained. Retry uses its original operation,
          revision and key.
        </p>
        <Button
          class="mt-3"
          size="sm"
          variant="outline"
          disabled={snapshot.busy ||
            snapshot.session?.mutationAllowed !== true ||
            snapshot.editor?.status === "writing" ||
            snapshot.editor?.status === "error"}
          onclick={() => void controller.retry().catch(() => {})}
          >Retry same mutation</Button
        >
      </Alert.Description></Alert.Root
    >
  {/if}
  {#if managed && snapshot.selected}
    <Button
      size="sm"
      variant="outline"
      disabled={snapshot.busy ||
        snapshot.session?.mutationAllowed !== true ||
        snapshot.editor?.status === "writing"}
      onclick={() => void restoreDraft()}>Restore saved draft and intent</Button
    >
  {/if}
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
    {loginUrl}
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
    {:else if page === "settings"}{#key ownerKey}<SettingsPage
          {snapshot}
          {controller}
          navigationBlocked={blockedNavigation}
          {restoreEpoch}
        />{/key}
    {:else if descriptor}{#key `${ownerKey}/${descriptor.kind}`}<ResourcePage
          {snapshot}
          {controller}
          {descriptor}
          {restoreEpoch}
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
