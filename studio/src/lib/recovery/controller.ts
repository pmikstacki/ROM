import { parseWire, stringifyWire } from "../client/codec.ts";
import type { Invocation, ProjectedView, WireValue } from "../client/types.ts";
import { failedAttempt } from "./outcome.ts";
import { editorProofValue, identifier, invocationValue, operationWire, principalKey, principalValue, revision } from "./record.ts";
import type { AcceptedIntent, IntentRecord } from "./record.ts";
import { copyView, snapshot } from "./snapshot.ts";
import { intentStorage } from "./storage.ts";
import type { MutationRecovery, MutationRecoveryOptions, MutationRecoveryState, RecoveryBinding, RecoveryPhase } from "./types.ts";

export function createMutationRecovery(options: MutationRecoveryOptions): MutationRecovery {
  const namespace = identifier(options.namespace), slot = identifier(options.slot);
  const target = { kind: identifier(options.target.kind), id: identifier(options.target.id) };
  const maxBytes = options.maxBytes ?? 1048576;
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 1) throw Error("invalid recovery byte limit");
  let binding: RecoveryBinding = { client: options.binding.client, principal: options.binding.principal ? principalValue(options.binding.principal) : null };
  const storage = intentStorage({ ...options, namespace, slot, target }, maxBytes);
  let record: IntentRecord | null = null, result: ProjectedView | null = null;
  let phase: RecoveryPhase = binding.principal ? "idle" : "quarantined";
  let error: MutationRecoveryState["error"] = null;
  let epoch = 0, disposed = false, active: AbortController | null = null;
  const listeners = new Set<(state: MutationRecoveryState) => void>();
  let observedGeneration = binding.client.generation;
  function observeGeneration() {
    if (observedGeneration !== binding.client.generation) {
      observedGeneration = binding.client.generation; epoch++; active?.abort(); active = null;
      result = null; error = null; if (phase === "submitting") phase = "unknown";
    }
  }
  const state = () => { observeGeneration(); return snapshot(record, phase, result, error, maxBytes); };
  function notify() { for (const listener of listeners) listener(state()); }
  function ticket() { observeGeneration(); return { epoch, client: binding.client, generation: binding.client.generation }; }
  function current(t: ReturnType<typeof ticket>) { return !disposed && t.epoch === epoch && t.client === binding.client && t.generation === binding.client.generation && binding.principal !== null; }
  function requireCurrent(t: ReturnType<typeof ticket>) { if (!current(t)) throw Error("recovery binding changed"); }
  function fresh(): IntentRecord {
    if (!binding.principal) throw Error("recovery principal unavailable");
    return { format: "rom-mutation-intent-v1", namespace, principal: { ...binding.principal }, target: { ...target }, draftWire: null, accepted: null };
  }
  function check(record: IntentRecord | null) {
    if (record && (record.namespace !== namespace || record.target.kind !== target.kind || record.target.id !== target.id)) throw Error("recovery scope mismatch");
    return !record || Boolean(binding.principal && principalKey(record.principal) === principalKey(binding.principal));
  }
  async function persist(next: IntentRecord | null, t: ReturnType<typeof ticket>) {
    requireCurrent(t);
    try { await storage.write(next); }
    catch { if (current(t)) { phase = "storage_error"; error = { category: "storage_error", reason: "Pending intent storage was not confirmed." }; notify(); } throw Error("recovery storage failed"); }
    requireCurrent(t);
    record = next;
  }
  const controller: MutationRecovery = {
    get state() { return state(); },
    subscribe(listener) { listeners.add(listener); listener(state()); return () => { listeners.delete(listener); }; },
    restore() {
      const t = ticket();
      if (!binding.principal) { phase = "quarantined"; record = null; result = null; error = null; notify(); return Promise.resolve(); }
      return storage.exclusive(async () => {
        requireCurrent(t);
        if (active) throw Error("recovery attempt pending");
        let next;
        try { next = await storage.read(); check(next); }
        catch { if (current(t)) { phase = "storage_error"; error = { category: "invalid_record", reason: "Pending intent could not be restored." }; notify(); } throw Error("invalid recovery record"); }
        requireCurrent(t);
        if (!check(next)) { record = null; result = null; error = null; phase = "quarantined"; notify(); return; }
        record = next; result = null; error = null; phase = next?.accepted?.phase ?? "idle"; notify();
      });
    },
    stage(operation) {
      const t = ticket();
      const wire = operation === null ? null : operationWire(operation, maxBytes);
      return storage.exclusive(async () => {
        requireCurrent(t);
        if (phase === "quarantined" || phase === "storage_error") throw Error("recovery lane unavailable");
        await persist({ ...(record ?? fresh()), draftWire: wire }, t);
        notify();
      });
    },
    begin(identity) {
      const t = ticket();
      const requestedEditorProof = identity.editorProof;
      return storage.exclusive(async () => {
        requireCurrent(t);
        const editorProof = requestedEditorProof === undefined ? undefined : editorProofValue(requestedEditorProof);
        if (active || phase === "quarantined" || phase === "storage_error" || state().hasUnresolvedIntent || !record?.draftWire) throw Error("resolve pending intent before beginning another mutation");
        const invocation: Invocation = { kind: target.kind, id: target.id, expected: revision(identity.expected), idempotency: identifier(identity.idempotency), operation: parseWire(record.draftWire, maxBytes) as Invocation["operation"] };
        if (identity.retryEpoch !== undefined) { revision(identity.retryEpoch); invocation.retry_epoch = identity.retryEpoch; }
        const wire = stringifyWire(invocation as unknown as WireValue, maxBytes);
        invocationValue(wire, maxBytes);
        const accepted: AcceptedIntent = { wire, draftWire: record.draftWire, attempted: false, uncertain: false, phase: "prepared", knowledge: "not_attempted" };
        if (editorProof !== undefined) accepted.editorProof = editorProof;
        await persist({ ...record, accepted }, t); phase = "prepared"; result = null; error = null; notify();
      });
    },
    async retry(signal) {
      const t = ticket();
      let accepted!: AcceptedIntent, priorUncertain = false;
      const transport = new AbortController();
      const abort = () => transport.abort(signal?.reason);
      if (signal?.aborted) abort(); else signal?.addEventListener("abort", abort, { once: true });
      try {
        await storage.exclusive(async () => {
          requireCurrent(t);
          if (active || phase === "quarantined" || phase === "storage_error" || !record?.accepted || ["committed", "not_committed"].includes(record.accepted.knowledge)) throw Error("no retryable recovery intent");
          if (transport.signal.aborted) throw Error("recovery attempt aborted");
          accepted = { ...record.accepted }; priorUncertain = accepted.uncertain;
          await persist({ ...record, accepted: { ...accepted, attempted: true, uncertain: true, phase: "unknown", knowledge: "unknown" } }, t);
          active = transport; phase = "submitting"; error = null; notify();
        });
        requireCurrent(t);
        let view: ProjectedView;
        try { view = await t.client.submit(t.client.prepare(invocationValue(accepted.wire, maxBytes)), transport.signal); }
        catch (problem) {
          requireCurrent(t);
          await storage.exclusive(async () => {
            requireCurrent(t);
            const outcome = failedAttempt(problem, priorUncertain);
            await persist({ ...record!, accepted: { ...record!.accepted!, phase: outcome.phase, attempted: true, uncertain: outcome.uncertain, knowledge: outcome.knowledge } }, t);
            phase = outcome.phase; error = outcome.error; notify();
          });
          requireCurrent(t);
          throw problem;
        }
        requireCurrent(t);
        await storage.exclusive(async () => {
          requireCurrent(t);
          await persist({ ...record!, draftWire: record!.draftWire === accepted.draftWire ? null : record!.draftWire, accepted: { ...record!.accepted!, phase: "succeeded", attempted: true, uncertain: false, knowledge: "committed" } }, t);
          result = copyView(view, maxBytes); phase = "succeeded"; error = null; notify();
        });
        requireCurrent(t);
        return copyView(view, maxBytes)!;
      } finally { signal?.removeEventListener("abort", abort); if (active === transport) active = null; }
    },
    async rebind(next) {
      const priorPrincipal = binding.principal ? principalKey(binding.principal) : null;
      epoch++; active?.abort(); active = null;
      binding = { client: next.client, principal: next.principal ? principalValue(next.principal) : null };
      observedGeneration = binding.client.generation;
      result = null; error = null;
      if (!binding.principal || priorPrincipal !== principalKey(binding.principal) || record && !check(record)) { record = null; phase = "quarantined"; notify(); return; }
      if (record?.accepted?.attempted && phase === "submitting") phase = "unknown";
      notify();
    },
    discard(discardOptions) {
      const t = ticket();
      return storage.exclusive(async () => {
        requireCurrent(t);
        if (phase === "quarantined" || active) throw Error("recovery lane unavailable");
        if ((record?.accepted?.attempted || phase === "storage_error") && !discardOptions.acknowledgePossibleCommit) throw Error("discard must acknowledge possible server commit");
        await persist(null, t); result = null; error = null; phase = "idle"; notify();
      });
    },
    dispose() { epoch++; disposed = true; active?.abort(); active = null; record = null; result = null; error = null; phase = "quarantined"; listeners.clear(); },
  };
  return controller;
}
