import type { WireValue } from "../client/types.ts";
import { stringifyWire } from "../client/codec.ts";
export function displayValue(value: WireValue | undefined): string {
  if (value === undefined) return "Absent";
  if (value === null) return "Null";
  if (typeof value === "string") return value;
  return stringifyWire(value);
}

/** Render only a bounded prefix of a value that cannot be edited inline. */
export function previewValue(value: WireValue, depth = 0): string {
  if (value === null) return "Null";
  if (typeof value === "string")
    return JSON.stringify(value.slice(0, 80)) + (value.length > 80 ? "…" : "");
  if (typeof value !== "object") return String(value);
  if (depth >= 3)
    return Array.isArray(value)
      ? `[${value.length} items]`
      : `{${Object.keys(value).length} entries}`;
  if (Array.isArray(value)) {
    const head = value.slice(0, 3).map((item) => previewValue(item, depth + 1));
    return `[${head.join(", ")}${value.length > 3 ? ", …" : ""}]`;
  }
  const entries = Object.entries(value);
  const head = entries
    .slice(0, 3)
    .map(([key, item]) =>
      `${JSON.stringify(key.slice(0, 40))}: ${previewValue(item, depth + 1)}`,
    );
  return `{${head.join(", ")}${entries.length > 3 ? ", …" : ""}}`;
}
