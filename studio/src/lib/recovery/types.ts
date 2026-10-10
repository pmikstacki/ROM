import type { Operation, ProjectedView, RomClient } from "../client/types.ts";
export interface DurablePrincipal { authority: string; kind: "embedded" | "human" | "service"; subject: string }
export interface StoredIntent { version: string; payload: string }
/**
 * The host selects storage and privacy policy; no default store is installed.
 * compareExchange must atomically check ownership and acknowledge durability before returning true.
 * namespace, slot, principal and staged operations must exclude credentials and session secrets.
 */
export interface PendingIntentStore {
  read(slot: string): Promise<StoredIntent | null>;
  compareExchange(slot: string, expectedVersion: string | null, next: StoredIntent | null): Promise<boolean>;
}
export interface RecoveryBinding { client: RomClient; principal: DurablePrincipal | null }
export interface MutationRecoveryOptions {
  namespace: string; slot: string; target: { kind: string; id: string }; binding: RecoveryBinding;
  store: PendingIntentStore; newVersion: () => string; maxBytes?: number;
}
export type RecoveryPhase = "idle" | "prepared" | "submitting" | "unknown" | "succeeded" | "rejected" | "conflict" | "quarantined" | "storage_error";
export type CommitKnowledge = "not_attempted" | "unknown" | "committed" | "not_committed";
export interface MutationRecoveryState {
  phase: RecoveryPhase; commitKnowledge: CommitKnowledge; hasDraft: boolean; hasUnresolvedIntent: boolean;
  draft: Operation | null; result: ProjectedView | null; error: { category: string; reason: string } | null;
  /** Exact accepted editor association; local cleanup metadata, never authority or a receipt. */
  acceptedEditorProof?: string;
}
export interface BeginMutation { expected: bigint | null; idempotency: string; retryEpoch?: bigint; editorProof?: string }
export interface MutationRecovery {
  readonly state: MutationRecoveryState;
  subscribe(listener: (state: MutationRecoveryState) => void): () => void;
  restore(): Promise<void>;
  stage(operation: Operation | null): Promise<void>;
  begin(identity: BeginMutation): Promise<void>;
  retry(signal?: AbortSignal): Promise<ProjectedView>;
  rebind(binding: RecoveryBinding): Promise<void>;
  discard(options: { acknowledgePossibleCommit: boolean }): Promise<void>;
  dispose(): void;
}
