import { sameOwner } from "./session-binding.ts";
import { createMutationRecovery } from "../recovery/controller.ts";
import type { MutationRecovery, RecoveryBinding } from "../recovery/types.ts";
import type { Operation, RomClient } from "../client/types.ts";
import type {
  ApplicationRecoverySnapshot,
  ManagedApplicationOptions,
  ResourceTarget,
} from "./session-types.ts";

/** One selected target uses the existing durable recovery contract. No receipt store lives here. */
export function createMutationLane(options: {
  recovery: ManagedApplicationOptions["recovery"];
  binding(): RecoveryBinding;
  allowed(): boolean;
  publish(snapshot: ApplicationRecoverySnapshot | null): void;
}) {
  let lane: MutationRecovery | null = null,
    target: ResourceTarget | null = null,
    owner: RecoveryBinding["principal"] = null;
  let unsubscribe: (() => void) | null = null,
    epoch = 0;
  function currentTarget(next: ResourceTarget) {
    return target?.kind === next.kind && target.id === next.id;
  }
  function available() {
    if (!options.allowed())
      throw Error("managed session authority unavailable");
  }
  function guardNavigation() {
    if (lane?.state.phase === "quarantined") return;
    if (
      lane?.state.hasUnresolvedIntent ||
      lane?.state.phase === "storage_error"
    )
      throw Error("Resolve the pending mutation before navigation.");
  }
  function select(next: ResourceTarget, localDraft = false) {
    if (!localDraft) available();
    const binding = options.binding();
    if (!binding.principal)
      throw Error("managed session principal unavailable");
    if (lane && currentTarget(next) && sameOwner(owner, binding.principal))
      return lane;
    guardNavigation();
    unsubscribe?.();
    lane?.dispose();
    epoch++;
    owner = { ...binding.principal };
    target = { ...next };
    lane = createMutationRecovery({
      namespace: options.recovery.namespace,
      slot: options.recovery.slot(binding.principal, next, "intent"),
      target: next,
      binding,
      store: options.recovery.intentStore,
      newVersion: options.recovery.newVersion,
      maxBytes: options.recovery.maxBytes,
    });
    const selected = lane,
      ticket = epoch;
    const stop = selected.subscribe((state) => {
      if (ticket === epoch && lane === selected)
        options.publish(
          state.phase === "quarantined" ? null : { target: { ...next }, state },
        );
    });
    if (ticket !== epoch || lane !== selected) {
      stop();
      throw Error("managed session or target changed");
    }
    unsubscribe = stop;
    return selected;
  }
  return {
    guardNavigation,
    async stage(next: ResourceTarget, operation: Operation | null) {
      const selected = select(next, true);
      await selected.stage(operation);
    },
    async mutate(
      next: ResourceTarget,
      expected: bigint | null,
      operation: Operation,
    ) {
      available();
      const selected = select(next),
        ticket = epoch;
      if (selected.state.hasUnresolvedIntent)
        throw Error(
          "Resolve the pending mutation before submitting another operation.",
        );
      await selected.stage(operation);
      if (ticket !== epoch) throw Error("managed session changed");
      available();
      await selected.begin({
        expected,
        idempotency: options.recovery.newCommandKey(),
        retryEpoch: options.recovery.retryEpoch(),
      });
      if (ticket !== epoch) throw Error("managed session changed");
      available();
      return selected.retry();
    },
    async retry() {
      available();
      if (!lane) throw Error("No unknown mutation to retry.");
      return lane.retry();
    },
    async restore(next: ResourceTarget) {
      const selected = select(next);
      await selected.restore();
    },
    async discard(acknowledgePossibleCommit: boolean) {
      available();
      if (!lane) return;
      await lane.discard({ acknowledgePossibleCommit });
    },
    rebind(binding: RecoveryBinding) {
      epoch++;
      unsubscribe?.();
      unsubscribe = null;
      if (!lane || !target) return Promise.resolve();
      const selected = lane,
        next = { ...target },
        ticket = epoch;
      // rebind clears authority-bound results synchronously before any observer publication.
      const pending = selected.rebind(binding);
      const stop = selected.subscribe((state) => {
        if (ticket === epoch && lane === selected)
          options.publish(
            state.phase === "quarantined" ? null : { target: next, state },
          );
      });
      if (ticket === epoch && lane === selected) unsubscribe = stop;
      else stop();
      return pending;
    },
    dispose() {
      epoch++;
      unsubscribe?.();
      unsubscribe = null;
      lane?.dispose();
      lane = null;
      target = null;
      owner = null;
      options.publish(null);
    },
  };
}
