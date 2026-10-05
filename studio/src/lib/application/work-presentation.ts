import type { WireObject, WireValue } from "../client/types.ts";
import { record } from "../client/validation.ts";
export const workStates = [
  "Pending",
  "Leased",
  "AwaitingReconciliation",
  "Done",
];
export function workStatus(value: WireValue): string {
  if (value === "AwaitingReconciliation") return "Awaiting reconciliation";
  if (value !== null && typeof value === "object" && !Array.isArray(value))
    return `Stopped · ${String(record(value).Stopped)}`;
  return String(value);
}
export function workDefinition(item: WireObject): string {
  return String(record(item.definition).name);
}
export function workQuery(
  category: string,
  state: string,
  definition: string,
): WireObject {
  return {
    limit: 50,
    ...(category ? { category } : {}),
    ...(state ? { state } : {}),
    ...(definition ? { definition } : {}),
  };
}
