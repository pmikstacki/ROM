import type {
  PendingIntentStore,
  DurablePrincipal,
  MutationRecoveryState,
} from "../recovery/types.ts";
import type { RomClient } from "../client/types.ts";
export interface ResourceTarget {
  kind: string;
  id: string;
}
export interface StudioAuthProfile {
  authority: string;
  now(): number;
  recovery: {
    namespace: string;
    intentStore: PendingIntentStore;
    editorStore: PendingIntentStore;
    slot(
      principal: DurablePrincipal,
      target: ResourceTarget,
      record: "intent" | "editor",
    ): string;
    /** Stable creation scope; distinct from every Resource-target slot. No stored value grants authority. */
    creationSlot?(
      principal: DurablePrincipal,
      kind: string,
      record: "intent" | "editor",
    ): string;
    newVersion(): string;
    newCommandKey(): string;
    retryEpoch(): bigint;
    maxBytes: number;
  };
}
export interface ManagedApplicationOptions {
  recovery: StudioAuthProfile["recovery"];
}
export interface ApplicationBinding {
  client: RomClient;
  principal: DurablePrincipal | null;
}
export interface ApplicationSessionState {
  status: "legacy" | "active" | "transient" | "renewing" | "denied" | "cleared";
  stale: boolean;
  mutationAllowed: boolean;
}
export type SessionRenewalOutcome = "fresh" | "transient" | "denied";
export interface ApplicationRecoverySnapshot {
  target: ResourceTarget;
  state: MutationRecoveryState;
}
