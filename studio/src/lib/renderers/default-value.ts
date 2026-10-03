import type { Shape, WireValue } from "../client/types.ts";
export function baseShape(shape: Shape): Shape {
  return shape.type === "optional" || shape.type === "nullable"
    ? baseShape(shape.value)
    : shape;
}
export function defaultValue(shape: Shape): WireValue {
  switch (shape.type) {
    case "optional":
    case "nullable":
      return defaultValue(shape.value);
    case "bool":
      return false;
    case "u64":
    case "i64":
      return 0n;
    case "f64":
      return 0;
    case "list":
      return [];
    case "map":
      return {};
    case "enum":
      return shape.value[0] ?? "";
    default:
      return "";
  }
}
