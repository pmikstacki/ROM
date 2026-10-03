<script lang="ts">
  import { onDestroy, onMount, untrack } from "svelte";
  import { createClient } from "./lib/client/client.ts";
  import type { RomClient } from "./lib/client/types.ts";
  import { createApplication } from "./lib/application/controller.ts";
  import ResourcePage from "./lib/application/ResourcePage.svelte";
  import WorkPage from "./lib/application/WorkPage.svelte";
  import { Button } from "./lib/components/ui/button/index.js";
  import { createBrowserAuth } from "./lib/application/auth.ts";
  import type { ProviderChoice } from "./lib/application/auth.ts";
  const base = import.meta.env.BASE_URL;
  const auth = createBrowserAuth(base);
  let {
    client = createClient({ base: `${base}api`, csrf: auth.csrf }),
  }: { client?: RomClient } = $props();
  let providers = $state<ProviderChoice[]>([]),
    authError = $state(""),
    checking = $state(false);
  let sessionGeneration: string | undefined;
  let destroyed = false;
  async function sessionCheck(connect = false) {
    if (checking) return;
    checking = true;
    try {
      const session = await auth.refresh();
      if (destroyed) return;
      if (!session.authenticated) {
        controller.disconnect();
        const choices = await auth.providers();
        providers = choices.providers;
        sessionGeneration = undefined;
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
      } else {
        try {
          sessionStorage.removeItem("rom-primary-redirect");
        } catch {}
        if (
          sessionGeneration !== undefined &&
          sessionGeneration !== session.generation
        )
          controller.disconnect();
        sessionGeneration = session.generation;
        providers = [];
        if (connect || controller.state.phase === "disconnected")
          await controller.connect();
      }
      authError = "";
    } catch (problem) {
      controller.disconnect();
      authError =
        problem instanceof Error ? problem.message : "Session unavailable.";
    } finally {
      checking = false;
    }
  }
  async function signOut() {
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
    page = $state<"resources" | "work">("resources");
  const unsubscribe = controller.subscribe((next) => (snapshot = next));
  onDestroy(() => {
    destroyed = true;
    unsubscribe();
    controller.disconnect();
  });
  let descriptor = $derived(
    snapshot.descriptors.find((item) => item.kind === snapshot.kind),
  );
</script>

<main class="studio-shell">
  <header>
    <h1>ROM Studio</h1>
    <p>One Resource definition. One mutation path.</p>
  </header>
  {#if snapshot.phase === "disconnected" || snapshot.phase === "error"}<section>
      <h2>Connect to ROM</h2>
      <p>Sign in through the configured identity provider before connecting.</p>
      {#each providers as provider}<a
          class="provider-link"
          href={auth.loginUrl(provider.id)}>Sign in with {provider.label}</a
        >{/each}
      <Button disabled={checking} onclick={() => void sessionCheck(true)}
        >Connect</Button
      >
    </section>{:else if snapshot.phase === "connecting"}<p role="status">
      Connecting to ROM…
    </p>{:else}
    <nav aria-label="Studio">
      <Button variant="outline" onclick={() => (page = "resources")}
        >Resources</Button
      ><Button variant="outline" onclick={() => (page = "work")}>Work</Button
      ><Button variant="outline" onclick={() => void signOut()}>Sign out</Button
      >
    </nav>
    <div class="studio-layout">
      <aside>
        <h2>Resources</h2>
        {#each snapshot.descriptors as item}<Button
            variant={snapshot.kind === item.kind ? "default" : "outline"}
            onclick={() => {
              page = "resources";
              void controller.selectKind(item.kind);
            }}>{item.kind}</Button
          >{/each}
      </aside>
      <section class="studio-content">
        {#if page === "work"}<WorkPage
            {snapshot}
            {controller}
          />{:else if descriptor}{#key descriptor.kind}<ResourcePage
              {snapshot}
              {controller}
              {descriptor}
            />{/key}{:else}<p>
            No Resources are available to this session.
          </p>{/if}
      </section>
    </div>
  {/if}
  {#if authError}<p role="alert">{authError}</p>{/if}
  {#if snapshot.error}<p role="alert">{snapshot.error}</p>{/if}
  {#if snapshot.pending}<section aria-label="Mutation outcome">
      <h2>Mutation outcome</h2>
      <p>
        {snapshot.pending.state} · {snapshot.pending.request.kind}/{snapshot
          .pending.request.id}
      </p>
      {#if snapshot.pending.state === "unknown"}<p>
          The result is unknown. Retry uses the same operation and idempotency
          key.
        </p>
        <Button
          disabled={snapshot.busy}
          onclick={() => void controller.retry().catch(() => {})}
          >Retry same mutation</Button
        >{/if}
    </section>{/if}
</main>
