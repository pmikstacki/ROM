<script lang="ts">
  import { onMount } from 'svelte';
  import { Button, Badge, openIndexedDbIntentStore, stringifyWire } from 'rom-studio';
  import type { IndexedDbIntentStore, WireValue } from 'rom-studio';
  import { createFlowRecovery, progressText, validateRunView, allowsRecovery } from './recovery.mjs';
  import { createBinding, readBoundView } from './binding.mjs';
  import { post, principal } from './transport.ts';
  const domain = new URLSearchParams(location.search).get('domain') === 'triage' ? 'triage' : 'publication';
  const binding = createBinding();
  const slot = `${domain}:browser-run`;
  let loaded = $state(false), view = $state<any>(null), recovery = $state<any>(null), message = $state(''), busy = $state(false), loggedOut = $state(false);
  let store: IndexedDbIntentStore, lane: ReturnType<typeof createFlowRecovery>;
  const sourceLabel = domain === 'publication' ? 'Captured article publication' : 'Ticket classification';
  onMount(() => {
    const ticket = binding.ticket();
    void (async () => {
      const opened = await openIndexedDbIntentStore({ name: 'rom-ai-flow-progress-fixture', maxBytes: 8192, maxSlots: 8, timeoutMs: 5000 });
      if (!binding.current(ticket)) { opened.close(); return; }
      store = opened;
      lane = createFlowRecovery({ domain, run: 'browser-run', principal, slot, store, post, newVersion: () => crypto.randomUUID(), changed: (value: any) => { if (binding.current(ticket)) recovery = value; } });
      await lane.restore();
      if (!binding.current(ticket)) { lane.quarantine(); store.close(); return; }
      await refresh(); if (binding.current(ticket)) loaded = true;
    })().catch(() => { if (binding.current(ticket)) { message = 'Flow initialization refused'; loaded = true; } });
    return () => { binding.dispose(); lane?.quarantine(); store?.close(); };
  });
  async function refresh(durableOnly = false) {
    if (loggedOut || !binding.current(binding.ticket())) return;
    view = null;
    await readBoundView(binding, () => post('view', { durable_only: durableOnly }), value => { view = validateRunView(value); message = ''; }, error => { message = error instanceof Error ? error.message : 'Progress unavailable'; });
  }
  async function operate(action: 'resume' | 'cancel') {
    const ticket = binding.ticket(); if (!binding.current(ticket) || loggedOut) return;
    busy = true; view = null; message = '';
    try {
      const response = await post('view', {});
      if (!binding.current(ticket) || loggedOut) return;
      const current = validateRunView(response);
      await lane.begin(action, current, `browser-${action}-${crypto.randomUUID()}`);
      if (!binding.current(ticket) || loggedOut) return;
      const responseView = await lane.retry();
      if (!binding.current(ticket) || loggedOut) return;
      view = responseView;
      message = action === 'cancel' ? 'Cancellation recorded; uncertain earlier effects remain unresolved' : 'Recovery admission acknowledged';
    } catch (error) { if (binding.current(ticket) && !loggedOut) message = error instanceof Error ? error.message : 'Recovery outcome unknown'; }
    finally { if (binding.current(ticket) && !loggedOut) busy = false; }
  }
  async function retry() {
    const ticket = binding.ticket(); if (!binding.current(ticket) || loggedOut) return;
    busy = true; view = null;
    try { const responseView = await lane.retry(); if (!binding.current(ticket) || loggedOut) return; view = responseView; message = 'Original recovery operation acknowledged'; }
    catch (error) { if (binding.current(ticket) && !loggedOut) message = error instanceof Error ? error.message : 'Recovery outcome unknown'; }
    finally { if (binding.current(ticket) && !loggedOut) busy = false; }
  }
  async function logout() {
    loggedOut = true; binding.quarantine(); view = null; busy = false; lane?.quarantine(); recovery = lane?.state;
    message = 'Signed out; earlier operation outcome is retained';
    try { await post('logout', {}); } catch { /* Local quarantine survives transport refusal. */ }
  }
</script>

<main class="mx-auto max-w-2xl space-y-5 p-6">
  <h1 class="text-xl font-semibold">{sourceLabel}</h1>
  <p>Progress describes committed stages and advisory read activity. Recovery checks current permission and revision again.</p>
  <output data-testid="loaded">{loaded ? 'ready' : 'loading'}</output>
  {#if view}
    <section aria-label="Authorized flow progress" class="space-y-3">
      <Badge variant="outline"><span data-testid="state">{typeof view.state === 'string' ? view.state : 'Waiting'}</span></Badge>
      <p data-testid="read-progress">{progressText(view.read_progress)}</p>
      {#if view.read_progress}<p>Read attempt <span data-testid="read-ordinal">{String(view.read_progress.ordinal)}</span></p>{/if}
      <p>Revision <span data-testid="revision">{view.revision}</span></p>
      <ol aria-label="Committed milestones">{#each view.milestones as milestone}<li>{typeof milestone.state === 'string' ? milestone.state : 'Waiting'}</li>{/each}</ol>
      {#if view.failure}<p role="alert" data-testid="failure">{typeof view.failure === 'string' ? view.failure : 'Run refused'}</p>{/if}
      {#if view.output !== null}<output data-testid="committed-output">{stringifyWire(view.output as WireValue)}</output>{/if}
      <p data-testid="generation-attempts">Generations: {String(view.counters.generation_attempts)}</p>
    </section>
  {/if}
  <div class="flex flex-wrap gap-2">
    <Button onclick={() => void refresh()} disabled={busy || loggedOut}>Refresh progress</Button>
    <Button variant="outline" onclick={() => void refresh(true)} disabled={busy || loggedOut}>Inspect durable activity</Button>
    <Button onclick={() => void operate('resume')} disabled={!allowsRecovery(view) || busy || loggedOut || recovery?.pending}>Recover read</Button>
    <Button variant="outline" onclick={() => void retry()} disabled={busy || loggedOut || !recovery?.pending}>Retry original operation</Button>
    <Button variant="outline" onclick={() => void operate('cancel')} disabled={!view || busy || loggedOut || recovery?.pending}>Request cancellation</Button>
    <Button variant="outline" onclick={() => void logout()} disabled={loggedOut}>Sign out</Button>
  </div>
  <p role="status" data-testid="message">{message}</p>
  <p data-testid="operation-knowledge">{recovery?.knowledge ?? 'not_attempted'}</p>
  <p data-testid="operation-phase">{recovery?.phase ?? 'idle'}</p>
  <p>Transient narration is not saved as a committed result.</p>
</main>
