import {
  createCreationDrafts,
  creationDraftMatches,
} from "./creation-drafts.ts";
import { editorFingerprint } from "../recovery/editor-fingerprint.ts";
import type { CreationDraft, CreationDraftState } from "./creation-drafts.ts";
import { createMutationRecovery } from "../recovery/controller.ts";
import { restoreMutationRecovery } from "../recovery/restore.ts";
import type { MutationRecovery, RecoveryBinding } from "../recovery/types.ts";
import type { Operation, WireValue } from "../client/types.ts";
import { sameOwner } from "./session-binding.ts";
import { identifier } from "../recovery/record.ts";
import type {
  ApplicationRecoverySnapshot,
  StudioAuthProfile,
} from "./session-types.ts";

export interface CreationWorkflowState {
  editor: CreationDraftState;
  recovery: ApplicationRecoverySnapshot | null;
  busy: boolean;
}
/** Framework-owned creation workflow; accepted commands remain in the shared receipt recovery contract. */
export function createCreationWorkflow(options: {
  kind: string;
  descriptor: string;
  recovery: StudioAuthProfile["recovery"];
  binding(): RecoveryBinding;
  allowed(): boolean;
  draftAllowed?(): boolean;
  publish(state: CreationWorkflowState): void;
}) {
  const kind = identifier(options.kind),
    recovery = options.recovery;
  if (!recovery.creationSlot)
    throw Error("Creation recovery requires an explicit creation scope.");
  const creationSlot = recovery.creationSlot;
  let owner = options.binding().principal,
    epoch = 0,
    busy = false,
    disposed = false;
  let lane: MutationRecovery | null = null,
    target: { kind: string; id: string } | null = null;
  let stop: (() => void) | null = null;
  const canDraft = () => (options.draftAllowed ?? options.allowed)();
  const visible = () =>
    !disposed && canDraft() && sameOwner(owner, options.binding().principal);
  const editors = createCreationDrafts({
    namespace: recovery.namespace,
    kind,
    descriptor: options.descriptor,
    maxBytes: recovery.maxBytes,
    principal: () => options.binding().principal,
    slot: (principal) => creationSlot(principal, kind, "editor"),
    store: recovery.editorStore,
    newVersion: recovery.newVersion,
    publish: () => publish(),
  });
  function snapshot(): CreationWorkflowState {
    if (!visible())
      return {
        editor: { status: "idle", snapshot: null },
        recovery: null,
        busy: false,
      };
    const state = lane?.state;
    return {
      editor: editors.state,
      busy,
      recovery:
        state && target && state.phase !== "quarantined"
          ? { target: { ...target }, state }
          : null,
    };
  }
  function publish() {
    options.publish(snapshot());
  }
  function check(ticket: number, authority = true) {
    if (
      disposed ||
      ticket !== epoch ||
      !sameOwner(owner, options.binding().principal)
    )
      throw Error("creation binding changed");
    if (authority && !options.allowed())
      throw Error("creation authority unavailable");
  }
  function attach(
    next: MutationRecovery,
    nextTarget: { kind: string; id: string },
  ) {
    stop?.();
    lane?.dispose();
    lane = next;
    target = { ...nextTarget };
    stop = next.subscribe(() => publish());
  }
  async function discover(ticket: number) {
    check(ticket);
    const binding = options.binding();
    const found = await restoreMutationRecovery({
      namespace: recovery.namespace,
      slot: creationSlot(binding.principal!, kind, "intent"),
      kind,
      operation: "create",
      binding,
      store: recovery.intentStore,
      newVersion: recovery.newVersion,
      maxBytes: recovery.maxBytes,
    });
    try {
      check(ticket);
    } catch (problem) {
      found?.recovery.dispose();
      throw problem;
    }
    if (found) attach(found.recovery, found.target);
  }
  async function exclusive<T>(
    operation: (ticket: number) => Promise<T>,
  ): Promise<T> {
    if (busy) throw Error("creation operation pending");
    const ticket = epoch;
    check(ticket);
    busy = true;
    publish();
    try {
      return await operation(ticket);
    } finally {
      if (ticket === epoch) {
        busy = false;
        publish();
      }
    }
  }
  async function retry(ticket: number) {
    check(ticket);
    if (!lane) throw Error("No creation intent to retry.");
    const result = await lane.retry();
    check(ticket);
    const saved = editors.state.snapshot,
      proof = lane.state.acceptedEditorProof;
    if (
      saved &&
      proof &&
      (await editorFingerprint(
        saved as unknown as WireValue,
        recovery.maxBytes,
      )) === proof
    ) {
      check(ticket);
      await editors.discard(saved);
      check(ticket);
    }
    return result;
  }
  return {
    get state() {
      return snapshot();
    },
    guardNavigation() {
      if (!sameOwner(owner, options.binding().principal)) return;
      editors.guardNavigation();
      if (
        busy ||
        (lane?.state.phase !== "quarantined" &&
          (lane?.state.hasUnresolvedIntent ||
            lane?.state.phase === "storage_error"))
      )
        throw Error("Resolve pending creation before navigation.");
    },
    async stage(value: CreationDraft) {
      check(epoch, false);
      if (!canDraft()) throw Error("creation draft authority unavailable");
      return editors.stage(value);
    },
    restore() {
      return exclusive(async (ticket) => {
        if (lane?.state.hasUnresolvedIntent)
          throw Error("Resolve pending creation before restoration.");
        await discover(ticket);
        check(ticket);
        await editors.restore();
        check(ticket);
      });
    },
    submit(id: string, operation: Operation) {
      return exclusive(async (ticket) => {
        if (operation.type !== "create")
          throw Error("Creation requires a create operation.");
        editors.guardNavigation();
        if (!lane) await discover(ticket);
        check(ticket);
        if (
          lane?.state.hasUnresolvedIntent ||
          lane?.state.phase === "storage_error"
        )
          throw Error(
            "Resolve pending creation before submitting another operation.",
          );
        if (lane) {
          await lane.discard({ acknowledgePossibleCommit: true });
          check(ticket);
          stop?.();
          stop = null;
          lane.dispose();
          lane = null;
          target = null;
        }
        const binding = options.binding(),
          nextTarget = { kind, id: identifier(id) };
        const next = createMutationRecovery({
          namespace: recovery.namespace,
          slot: creationSlot(binding.principal!, kind, "intent"),
          target: nextTarget,
          binding,
          store: recovery.intentStore,
          newVersion: recovery.newVersion,
          maxBytes: recovery.maxBytes,
        });
        attach(next, nextTarget);
        check(ticket);
        const saved = editors.state.snapshot;
        const editorProof =
          saved && creationDraftMatches(saved, id, operation)
            ? await editorFingerprint(
                saved as unknown as WireValue,
                recovery.maxBytes,
              )
            : undefined;
        check(ticket);
        await next.stage(operation);
        check(ticket);
        await next.begin({
          expected: null,
          idempotency: recovery.newCommandKey(),
          retryEpoch: recovery.retryEpoch(),
          ...(editorProof ? { editorProof } : {}),
        });
        check(ticket);
        return retry(ticket);
      });
    },
    retry() {
      return exclusive(retry);
    },
    async discard(acknowledgePossibleCommit: boolean) {
      return exclusive(async (ticket) => {
        if (lane) {
          await lane.discard({ acknowledgePossibleCommit });
          check(ticket);
        }
        await editors.discard();
        check(ticket);
      });
    },
    async rebind(binding: RecoveryBinding) {
      const same = sameOwner(owner, binding.principal);
      epoch++;
      busy = false;
      owner = binding.principal ? { ...binding.principal } : null;
      editors.rebind(owner);
      if (lane) await lane.rebind(binding);
      if (!same) {
        stop?.();
        stop = null;
        lane?.dispose();
        lane = null;
        target = null;
      }
      publish();
    },
    dispose() {
      disposed = true;
      epoch++;
      stop?.();
      stop = null;
      lane?.dispose();
      lane = null;
      owner = null;
      target = null;
      editors.rebind(null);
    },
  };
}
