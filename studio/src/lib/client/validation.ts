import type { ProjectedView, WireObject, WireValue } from "./types.ts";
export function record(value: WireValue): WireObject {
  if (value === null || typeof value !== "object" || Array.isArray(value))
    throw new Error("invalid object");
  return value;
}
export function text(value: WireValue | undefined): string {
  if (typeof value !== "string" || value.length === 0 || value.length > 4096)
    throw new Error("invalid text");
  return value;
}
export function unsigned(value: WireValue | undefined): bigint {
  if (
    typeof value !== "bigint" &&
    !(typeof value === "number" && Number.isSafeInteger(value))
  )
    throw new Error("invalid integer");
  const result = BigInt(value);
  if (result < 0n || result > 18446744073709551615n)
    throw new Error("integer bounds");
  return result;
}
export function projected(
  value: WireValue,
  kind: string,
  id?: string,
): ProjectedView {
  const object = record(value),
    key = record(object.key);
  const actualKind = text(key.kind),
    actualId = text(key.id);
  if (actualKind !== kind || (id !== undefined && actualId !== id))
    throw new Error("response identity mismatch");
  if (!Object.hasOwn(object, "value"))
    throw new Error("missing projected value");
  return {
    key: { kind: actualKind, id: actualId },
    revision: unsigned(object.revision),
    value: object.value === null ? null : record(object.value),
  };
}
export function projectedRows(
  value: WireValue,
  kind: string,
  maxRows: number,
): ProjectedView[] {
  if (!Array.isArray(value)) throw new Error("invalid rows");
  if (value.length > maxRows) throw new Error("rows limit");
  const rows = value.map((row) => projected(row, kind));
  if (new Set(rows.map((row) => row.key.id)).size !== rows.length)
    throw new Error("duplicate row identity");
  return rows;
}
