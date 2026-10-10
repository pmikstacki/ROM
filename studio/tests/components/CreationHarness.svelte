<script lang="ts">
  import { onMount } from "svelte";
  import ResourcePage from "../../src/lib/application/ResourcePage.svelte";
  import { createApplication } from "../../src/lib/application/controller.ts";
  import { createStudioBootstrap } from "../../src/lib/application/bootstrap.ts";
  import { createClient } from "../../src/client.ts";
  import type { ApplicationState } from "../../src/lib/application/controller.ts";
  let application = $state<ReturnType<typeof createApplication> | null>(null);
  let snapshot = $state.raw<ApplicationState | null>(null);
  onMount(() => {
    let close = () => {},
      stop = () => {},
      disposed = false;
    void (async () => {
      const stores = await createStudioBootstrap({
        version: 1,
        authority: "creation-fixture",
        recovery: {
          namespace: "creation-fixture",
          retryEpoch: "18446744073709551615",
          maxBytes: 65536,
          intentStore: {
            name: "creation-intents",
            maxBytes: 69632,
            maxSlots: 16,
            timeoutMs: 2000,
          },
          editorStore: {
            name: "creation-editors",
            maxBytes: 69632,
            maxSlots: 16,
            timeoutMs: 2000,
          },
        },
      });
      close = stores.close;
      if (disposed) return close();
      const client = createClient({ base: "/rom-studio/creation-api" });
      const app = createApplication(
        client,
        () => crypto.randomUUID(),
        undefined,
        stores.profile,
      );
      application = app;
      stop = app.subscribe((next) => (snapshot = next));
      await app.rebindSession({
        client,
        principal: {
          authority: "creation-fixture",
          kind: "human",
          subject: "alice",
        },
      });
    })();
    return () => {
      disposed = true;
      stop();
      application?.disconnect();
      close();
    };
  });
</script>

{#if application && snapshot?.descriptors[0]}
  <button
    onclick={() =>
      void application!.rebindSession({
        client: createClient({ base: "/rom-studio/creation-api" }),
        principal: {
          authority: "creation-fixture",
          kind: "human",
          subject: "alice",
        },
      })}>Renew creation session</button
  >
  <ResourcePage
    controller={application}
    {snapshot}
    descriptor={snapshot.descriptors[0]}
  />
{/if}
