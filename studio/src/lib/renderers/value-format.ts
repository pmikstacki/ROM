import type { WireValue } from "../client/types.ts";
import { stringifyWire } from "../client/codec.ts";
export function displayValue(value: WireValue | undefined): string {
  if (value === undefined) return "Absent";
  if (value === null) return "Null";
  if (typeof value === "string") return value;
  return stringifyWire(value);
}
