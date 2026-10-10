<script lang="ts">
  import { onMount } from 'svelte';
  import { Button, Label, Textarea } from 'rom-studio/controls';
  import { createMutationRecovery } from 'rom-studio/recovery';
  import type { MutationRecovery, MutationRecoveryState, Operation } from 'rom-studio/client';
  import { stringifyWire } from 'rom-studio/client';
  import { openHostStorage } from './storage.ts';
  import { control, owner, other, transport } from './session.ts';
  const query = new URLSearchParams(location.search), id = query.get('id') ?? 'missing', inventory = query.get('profile') === 'inventory';
  const slot = `${inventory ? 'inventory' : 'notes'}:${id}`;
  let lane: MutationRecovery, storage: Awaited<ReturnType<typeof openHostStorage>>, client: ReturnType<typeof transport>;
  let editor = $state(''), textarea = $state<HTMLTextAreaElement | null>(null), revision = $state<bigint | null>(null);
  let status = $state<MutationRecoveryState | null>(null), loaded = $state(false), message = $state(''), view = $state('editor'), principal = $state('alice');
  let generation = 0, edits: Promise<void> = Promise.resolve(), pending: Promise<unknown> | null = null;
  const valid = () => inventory ? editor.startsWith('SKU:') && editor.length > 4 : editor.length > 0 && editor !== '!';
  const operation = (): Operation => ({ type: 'action', input: { name: 'save', input: { title: editor, count: 9007199254740993n } } });
  onMount(() => {
    let disposed = false, unsubscribe: (() => void) | undefined;
    void (async () => {
      storage = await openHostStorage(); generation = Number((await control('session-state')).generation);
      client = transport(generation); const row = await client.read('recovery-notes', id); revision = row.revision;
      editor = await storage.readDraft(slot) ?? (inventory ? 'SKU:initial' : 'initial');
      lane = createMutationRecovery({ namespace:'recovery-browser', slot, target:{kind:'recovery-notes',id}, binding:{client,principal:owner}, store:storage.intents, newVersion:() => crypto.randomUUID() });
      if (disposed) { lane.dispose(); return; }
      unsubscribe = lane.subscribe(state => { status = state; }); await lane.restore(); loaded = true;
    })().catch(() => { message = 'Host initialization failed'; });
    return () => { disposed = true; unsubscribe?.(); lane?.dispose(); };
  });
  function stage() {
    const text = editor, validText = valid(), draft = validText ? operation() : null;
    edits = edits.then(async () => { await storage.writeDraft(slot, text); await lane.stage(draft); });
    void edits.catch(() => { message = 'Draft persistence failed'; });
  }
  async function save() {
    message = ''; await edits;
    if (!valid()) { message = 'Invalid draft retained'; return; }
    await lane.stage(operation());
    await lane.begin({ expected: revision, idempotency: `save-${id}-${crypto.randomUUID()}`, retryEpoch:0n });
    await attempt();
  }
  async function attempt() {
    pending = lane.retry();
    try { const result = await pending; revision = (result as {revision:bigint}).revision; message = 'Save confirmed'; }
    catch { message = 'Save outcome unresolved'; }
    finally { pending = null; }
  }
  async function navigate() {
    await edits; await pending?.catch(() => {});
    if (status?.hasUnresolvedIntent || status?.hasDraft || !valid()) { message = inventory ? 'Inventory changes retained' : 'Draft retained'; textarea?.focus(); return; }
    view = inventory ? 'inventory-overview' : 'notes-list';
  }
  async function refresh() {
    generation = Number((await control('session')).generation); client.invalidateSession(); client = transport(generation,principal);
    await lane.rebind({client,principal:principal === 'alice' ? owner : other}); message = 'Session renewed';
  }
  async function switchPrincipal() {
    principal = principal === 'alice' ? 'bob' : 'alice';
    client.invalidateSession(); client = transport(generation,principal);
    await lane.rebind({client,principal:principal === 'alice' ? owner : other}); editor = ''; message = 'Principal changed';
  }
  async function restoreOwner() { await lane.restore(); if (principal === 'alice') editor = await storage.readDraft(slot) ?? ''; }
</script>
<main>
  <h1>{inventory ? 'Inventory note editor' : 'Notes editor'}</h1>
  <output data-testid="loaded">{loaded ? 'ready' : 'loading'}</output>
  <output data-testid="view">{view}</output>
  <output data-testid="principal">{principal}</output>
  {#if loaded && view === 'editor'}
    <Label for="draft">{inventory ? 'Stock note' : 'Note text'}</Label>
    <Textarea id="draft" bind:value={editor} bind:ref={textarea} oninput={stage} aria-invalid={!valid()} disabled={principal !== 'alice'} />
    <Button onclick={() => { void save().catch(() => { message = 'Save admission refused'; }); }} disabled={principal !== 'alice'}>Save</Button>
    <Button onclick={() => { void attempt(); }} disabled={principal !== 'alice'}>Retry accepted save</Button>
    <Button onclick={() => { void navigate(); }}>Navigate</Button>
    <Button onclick={() => { void refresh(); }}>Renew session</Button>
    <Button onclick={() => { void switchPrincipal(); }}>Switch principal</Button>
    <Button onclick={() => { void restoreOwner(); }}>Restore owner intent</Button>
  {/if}
  <output data-testid="phase">{status?.phase ?? 'loading'}</output>
  <output data-testid="knowledge">{status?.commitKnowledge ?? 'loading'}</output>
  <output data-testid="draft-wire">{status?.draft ? stringifyWire(status.draft as never) : ''}</output>
  <output data-testid="result">{status?.result ? stringifyWire(status.result as never) : ''}</output>
  <output data-testid="message">{message}</output>
</main>
