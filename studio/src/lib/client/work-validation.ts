import { record, text, unsigned } from "./validation.ts";
import { stringifyWire } from "./codec.ts";
import type { WireObject, WireValue } from "./types.ts";
const stopped = new Set([
  "Depth",
  "WorkBudget",
  "Attempts",
  "Age",
  "Fanout",
  "Denied",
  "Conflict",
  "Invalid",
  "Missing",
  "DefinitionChanged",
  "CallbackPanicked",
  "Unavailable",
  "DeliveryPermanent",
]);
function identifier(value: WireValue | undefined, max = 1024): string {
  const result = text(value);
  if (new TextEncoder().encode(result).length > max)
    throw Error("operator identifier limit");
  return result;
}
function handle(value: WireValue | undefined): string {
  const result = text(value);
  if (!/^[0-9a-f]{64}$/.test(result)) throw Error("invalid work handle");
  return result;
}
function bool(value: WireValue | undefined): boolean {
  if (typeof value !== "boolean") throw Error("invalid boolean");
  return value;
}
function u32(value: WireValue | undefined): number {
  const result = unsigned(value);
  if (result > 4294967295n) throw Error("integer bounds");
  return Number(result);
}
function protocol(o: WireObject): void {
  if (o.protocol_version !== 1) throw Error("unsupported operator version");
}
function version(value: WireValue): WireObject {
  const v = record(value);
  return {
    generation: identifier(v.generation, 128),
    revision: unsigned(v.revision),
  };
}
function state(value: WireValue, allowed: string[]): WireValue {
  if (typeof value === "string" && allowed.includes(value)) return value;
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    const o = record(value);
    if (
      Object.keys(o).length === 1 &&
      typeof o.Stopped === "string" &&
      stopped.has(o.Stopped)
    )
      return { Stopped: o.Stopped };
  }
  throw Error("invalid work state");
}
function key(value: WireValue): WireValue {
  if (value === null) return null;
  const k = record(value);
  return { kind: text(k.kind), id: text(k.id) };
}
function view(value: WireValue): WireObject {
  const o = record(value);
  protocol(o);
  const definition = record(o.definition);
  if (o.category !== "Reaction" && o.category !== "Notification")
    throw Error("invalid work category");
  if (
    o.delivery !== null &&
    (![
      "Accepted",
      "Retryable",
      "Permanent",
      "Unknown",
      "TimedOut",
      "Panicked",
    ].includes(String(o.delivery)) ||
      typeof o.delivery !== "string")
  )
    throw Error("invalid delivery");
  return {
    protocol_version: 1,
    handle: handle(o.handle),
    version: version(o.version),
    category: o.category,
    definition: {
      name: identifier(definition.name),
      version: u32(definition.version),
    },
    state: state(o.state, [
      "Pending",
      "Leased",
      "AwaitingReconciliation",
      "Done",
    ]),
    attempts: u32(o.attempts),
    due: unsigned(o.due),
    delivery: o.delivery,
    source: key(o.source),
    target: key(o.target),
  };
}
function operation(value: WireValue): WireValue {
  if (value === "Retry") return value;
  const o = record(value);
  if (Object.keys(o).length !== 1 || !Object.hasOwn(o, "Reconcile"))
    throw Error("invalid work operation");
  const reconcile = record(o.Reconcile);
  if (Object.keys(reconcile).some((k) => k !== "evidence_ref"))
    throw Error("invalid work operation");
  return {
    Reconcile: {
      evidence_ref:
        reconcile.evidence_ref === undefined || reconcile.evidence_ref === null
          ? null
          : identifier(reconcile.evidence_ref),
    },
  };
}
export function workResponse(
  route: "capabilities" | "list" | "read" | "control",
  value: WireValue,
  request: WireObject,
  maxRows: number,
): WireObject {
  const o = record(value);
  protocol(o);
  if (route === "capabilities")
    return {
      protocol_version: 1,
      inspect: bool(o.inspect),
      retry: bool(o.retry),
      reconcile: bool(o.reconcile),
    };
  if (route === "read") {
    const result = view(value);
    if (result.handle !== handle(request.handle))
      throw Error("work identity mismatch");
    return result;
  }
  if (route === "list") {
    if (
      !Array.isArray(o.records) ||
      o.records.length > maxRows ||
      unsigned(request.limit) < BigInt(o.records.length)
    )
      throw Error("work records limit");
    const records = o.records.map(view);
    if (new Set(records.map((r) => r.handle)).size !== records.length)
      throw Error("duplicate work identity");
    return {
      protocol_version: 1,
      records,
      cursor: o.cursor === null ? null : identifier(o.cursor, 2048),
    };
  }
  const actual = version(o.version),
    expected = version(request.expected),
    op = operation(o.operation),
    requested = operation(request.operation);
  if (
    handle(o.handle) !== handle(request.handle) ||
    identifier(o.key) !== identifier(request.key) ||
    actual.generation !== expected.generation ||
    stringifyWire(op) !== stringifyWire(requested)
  )
    throw Error("work control correspondence");
  const outcome = state(o.outcome, ["Scheduled", "Completed", "Unresolved"]),
    replayed = bool(o.replayed);
  const revision = actual.revision as bigint,
    prior = expected.revision as bigint;
  if (outcome === "Unresolved") {
    if (requested === "Retry" || replayed || revision !== prior)
      throw Error("incoherent work revision");
  } else {
    if (
      (requested === "Retry" && outcome !== "Scheduled") ||
      prior === 18446744073709551615n ||
      revision !== prior + 1n
    )
      throw Error("incoherent work revision");
  }
  return {
    protocol_version: 1,
    handle: o.handle,
    version: actual,
    key: o.key,
    operation: op,
    outcome,
    replayed,
  };
}
