import type { Shape } from "./types.ts";
/** Labels change presentation only; membership always comes from Shape. */
export function enumLeaf(
  shape: Shape,
): Extract<Shape, { type: "enum" }> | undefined {
  let current = shape;
  for (let depth = 0; depth <= 16; depth++) {
    if (current.type === "enum") return current;
    if (
      current.type === "optional" ||
      current.type === "nullable" ||
      current.type === "list" ||
      current.type === "map"
    )
      current = current.value;
    else return undefined;
  }
  return undefined;
}
export function enumLabel(
  member: string,
  labels?: Record<string, string>,
): string {
  return labels && Object.hasOwn(labels, member) ? labels[member] : member;
}
