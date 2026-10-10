import { parseWire, stringifyWire } from "../client/codec.ts";
import type { Operation, ProjectedView, WireValue } from "../client/types.ts";
import type { IntentRecord } from "./record.ts";
import type { MutationRecoveryState, RecoveryPhase } from "./types.ts";
export function copyView(value: ProjectedView | null, maxBytes: number): ProjectedView | null {
  return value === null ? null : parseWire(stringifyWire(value as unknown as WireValue, maxBytes), maxBytes) as unknown as ProjectedView;
}
export function snapshot(record: IntentRecord | null, phase: RecoveryPhase, result: ProjectedView | null, error: MutationRecoveryState["error"], maxBytes: number): MutationRecoveryState {
  const visible = phase !== "quarantined";
  return {
    phase, commitKnowledge: visible ? record?.accepted?.knowledge ?? "not_attempted" : "unknown",
    hasDraft: visible && record?.draftWire != null,
    hasUnresolvedIntent: phase === "quarantined" || phase === "storage_error" || Boolean(record?.accepted && !["committed", "not_committed"].includes(record.accepted.knowledge)),
    draft: visible && record?.draftWire ? parseWire(record.draftWire, maxBytes) as unknown as Operation : null,
    result: visible ? copyView(result, maxBytes) : null,
    error: visible && error ? { ...error } : null,
    ...(visible && record?.accepted?.editorProof !== undefined ? { acceptedEditorProof: record.accepted.editorProof } : {}),
  };
}
