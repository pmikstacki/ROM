import { floatingMember } from "../client/serialization.ts";
import { parseWire, stringifyWire } from "../client/codec.ts";
import type { Invocation, Operation, WireObject, WireValue } from "../client/types.ts";
import type { CommitKnowledge, DurablePrincipal } from "./types.ts";
export interface AcceptedIntent {
  wire: string; draftWire: string; attempted: boolean; uncertain: boolean;
  phase: "prepared" | "unknown" | "succeeded" | "rejected" | "conflict";
  knowledge: CommitKnowledge;
  editorProof?: string;
}
export interface IntentRecord {
  format: "rom-mutation-intent-v1"; namespace: string; principal: DurablePrincipal;
  target: { kind: string; id: string }; draftWire: string | null; accepted: AcceptedIntent | null;
}
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw Error("invalid recovery record");
  if (![null, Object.prototype].includes(Object.getPrototypeOf(value))) throw Error("invalid recovery object");
  return value as Record<string, unknown>;
}
function fields(value: unknown, required: string[], optional: string[] = []) {
  const data = object(value);
  if (required.some(key => !Object.hasOwn(data, key)) || Object.keys(data).some(key => !required.includes(key) && !optional.includes(key))) throw Error("invalid recovery fields");
  return data;
}
export function identifier(value: unknown): string {
  if (typeof value !== "string" || !value || value.length > 4096) throw Error("invalid recovery identifier");
  return value;
}
/** Closed local association format; this digest does not establish authority. */
export function editorProofValue(value: unknown): string {
  if (typeof value !== "string" || value.length !== 71 || !/^sha256:[a-f0-9]{64}$/.test(value)) throw Error("invalid recovery editor proof");
  return value;
}
export function principalValue(value: unknown): DurablePrincipal {
  const data = fields(value, ["authority", "kind", "subject"]);
  if (!["embedded", "human", "service"].includes(String(data.kind))) throw Error("invalid recovery principal");
  return { authority: identifier(data.authority), kind: data.kind as DurablePrincipal["kind"], subject: identifier(data.subject) };
}
export function principalKey(value: DurablePrincipal): string { return stringifyWire(principalValue(value) as unknown as WireObject); }
export function revision(value: unknown): bigint | null {
  if (value === null) return null;
  if (typeof value === "number" && Number.isSafeInteger(value) && !Object.is(value, -0)) value = BigInt(value);
  if (typeof value !== "bigint" || value < 0n || value > 18446744073709551615n) throw Error("invalid recovery revision");
  return value;
}
function operation(value: unknown): Operation {
  const data = object(value);
  if (data.type === "delete") fields(data, ["type"]);
  else {
    fields(data, ["type", "input"]);
    if (data.type === "create" || data.type === "replace") object(data.input);
    else if (data.type === "patch") {
      for (const update of Object.values(object(data.input))) {
        const field = object(update);
        if (field.op === "remove") fields(field, ["op"]);
        else if (field.op === "set") fields(field, ["op", "value"]);
        else throw Error("invalid recovery field update");
      }
    } else if (data.type === "action") identifier(fields(data.input, ["name", "input"]).name);
    else throw Error("invalid recovery operation");
  }
  return data as unknown as Operation;
}
export function operationWire(value: Operation, maxBytes: number): string {
  const wire = stringifyWire(value as unknown as WireValue, maxBytes);
  operation(parseWire(wire, maxBytes));
  return wire;
}
export function invocationValue(wire: string, maxBytes: number): Invocation {
  const data = fields(parseWire(wire, maxBytes), ["kind", "id", "expected", "idempotency", "operation"], ["retry_epoch"]);
  identifier(data.kind); identifier(data.id); identifier(data.idempotency); revision(data.expected);
  if (floatingMember(data, "expected") || floatingMember(data, "retry_epoch")) throw Error("invalid recovery integer token");
  if (Object.hasOwn(data, "retry_epoch") && revision(data.retry_epoch) === null) throw Error("invalid recovery epoch");
  operation(data.operation);
  return data as unknown as Invocation;
}
export function encodeRecord(record: IntentRecord, maxBytes: number): string { return stringifyWire(record as unknown as WireValue, maxBytes); }
export function decodeRecord(payload: string, maxBytes: number): IntentRecord {
  const data = fields(parseWire(payload, maxBytes), ["format", "namespace", "principal", "target", "draftWire", "accepted"]);
  if (data.format !== "rom-mutation-intent-v1") throw Error("unsupported recovery format");
  identifier(data.namespace); principalValue(data.principal);
  const target = fields(data.target, ["kind", "id"]); identifier(target.kind); identifier(target.id);
  if (data.draftWire !== null) {
    if (typeof data.draftWire !== "string") throw Error("invalid recovery draft");
    operation(parseWire(data.draftWire, maxBytes));
  }
  if (data.accepted !== null) {
    const accepted = fields(data.accepted, ["wire", "draftWire", "attempted", "uncertain", "phase", "knowledge"], ["editorProof"]);
    if (Object.hasOwn(accepted, "editorProof")) editorProofValue(accepted.editorProof);
    if (typeof accepted.wire !== "string" || typeof accepted.draftWire !== "string" || typeof accepted.attempted !== "boolean" || typeof accepted.uncertain !== "boolean") throw Error("invalid accepted intent");
    if (!["prepared", "unknown", "succeeded", "rejected", "conflict"].includes(String(accepted.phase)) || !["not_attempted", "unknown", "committed", "not_committed"].includes(String(accepted.knowledge))) throw Error("invalid recovery status");
    const invocation = invocationValue(accepted.wire, maxBytes);
    if (invocation.kind !== target.kind || invocation.id !== target.id || operationWire(invocation.operation, maxBytes) !== accepted.draftWire) throw Error("recovery command mismatch");
    if (accepted.phase === "prepared" && (accepted.attempted || accepted.uncertain || accepted.knowledge !== "not_attempted")) throw Error("recovery prepared mismatch");
    if (accepted.phase === "succeeded" && (!accepted.attempted || accepted.uncertain || accepted.knowledge !== "committed")) throw Error("recovery success mismatch");
    if (accepted.phase !== "prepared" && !accepted.attempted) throw Error("recovery attempt mismatch");
    if (accepted.uncertain && accepted.knowledge !== "unknown") throw Error("recovery uncertainty mismatch");
    if (accepted.phase === "unknown" && (!accepted.uncertain || accepted.knowledge !== "unknown")) throw Error("recovery unknown mismatch");
    if (["rejected", "conflict"].includes(String(accepted.phase)) && !((accepted.uncertain && accepted.knowledge === "unknown") || (!accepted.uncertain && accepted.knowledge === "not_committed"))) throw Error("recovery refusal mismatch");
  }
  return data as unknown as IntentRecord;
}
