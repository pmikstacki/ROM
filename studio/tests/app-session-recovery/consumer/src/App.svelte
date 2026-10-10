<script lang="ts">
  import { onMount } from 'svelte';
  import { App as StudioApp, createStudioBootstrap, type StudioAuthProfile } from 'rom-studio';
  import 'rom-studio/styles';
  let profile = $state<StudioAuthProfile | null>(null);
  let failed = $state(false);
  onMount(() => {
    let disposed = false;
    let close: (() => void) | undefined;
    const store = { maxBytes: 1052672, maxSlots: 64, timeoutMs: 10000 };
    void createStudioBootstrap({ version: 1, authority: 'recovery-fixture', recovery: {
      namespace: 'app-session-authoring-v1', retryEpoch: '0', maxBytes: 1048576,
      intentStore: { ...store, name: 'rom-app-session-recovery-intents-v1' },
      editorStore: { ...store, name: 'rom-app-session-recovery-editors-v1' }
    }}).then(bootstrap => {
      close = bootstrap.close;
      if (disposed) { bootstrap.close(); return; }
      profile = bootstrap.profile;
    }).catch(() => { if (!disposed) failed = true; });
    return () => { disposed = true; close?.(); };
  });
</script>
{#if profile}<StudioApp authProfile={profile} />{:else if failed}<p role="alert">Explicit fixture storage failed.</p>{:else}<p role="status">Opening explicit fixture storage…</p>{/if}
