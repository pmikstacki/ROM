import type {
  FieldDescriptor,
  FieldIntent,
  FieldUpdate,
  Shape,
  WireObject,
  WireValue,
} from "./types.ts";

function invalid(path: string, reason: string): never {
  throw Error(`${path}: ${reason}`);
}
function object(value: WireValue, path: string): WireObject {
  if (!value || typeof value !== "object" || Array.isArray(value))
    return invalid(path, "object required");
  if (![null, Object.prototype].includes(Object.getPrototypeOf(value)))
    return invalid(path, "plain object required");
  for (const member of Object.values(Object.getOwnPropertyDescriptors(value))) {
    if (member.enumerable && !Object.hasOwn(member, "value"))
      return invalid(path, "getters are unsupported");
  }
  return value;
}

export function normalizeValue(
  shape: Shape,
  value: WireValue,
  path = "field",
  depth = 0,
): WireValue {
  if (depth > 16) return invalid(path, "shape depth limit");
  const next = (inner: Shape, v: WireValue, name = path) =>
    normalizeValue(inner, v, name, depth + 1);
  switch (shape.type) {
    case "optional":
      return next(shape.value, value);
    case "nullable":
      return value === null ? null : next(shape.value, value);
    case "string":
      return typeof value === "string"
        ? value
        : invalid(path, "string required");
    case "bool":
      return typeof value === "boolean"
        ? value
        : invalid(path, "boolean required");
    case "reference":
      return typeof value === "string" && value.length
        ? value
        : invalid(path, "reference ID required");
    case "enum":
      return typeof value === "string" && shape.value.includes(value)
        ? value
        : invalid(path, "declared enum value required");
    case "u64":
    case "i64": {
      if (typeof value === "number" && !Number.isSafeInteger(value))
        return invalid(path, "safe integer required; use bigint");
      if (typeof value !== "number" && typeof value !== "bigint")
        return invalid(path, "integer required");
      const number = BigInt(value);
      const low = shape.type === "u64" ? 0n : -(1n << 63n);
      const high = shape.type === "u64" ? (1n << 64n) - 1n : (1n << 63n) - 1n;
      return number >= low && number <= high
        ? number
        : invalid(path, "integer out of range");
    }
    case "f64": {
      if (typeof value !== "number" && typeof value !== "bigint")
        return invalid(path, "finite number required");
      const number = Number(value);
      return Number.isFinite(number)
        ? number
        : invalid(path, "finite number required");
    }
    case "list":
      return Array.isArray(value)
        ? value.map((v, index) => next(shape.value, v, `${path}[${index}]`))
        : invalid(path, "list required");
    case "map": {
      const result: WireObject = Object.create(null);
      for (const [key, v] of Object.entries(object(value, path)))
        result[key] = next(shape.value, v, `${path}.${key}`);
      return result;
    }
    default:
      return invalid(path, "unsupported shape");
  }
}

function known(
  fields: FieldDescriptor[],
  intents: Record<string, FieldIntent>,
): void {
  const names = new Set(fields.map((field) => field.name));
  for (const name of Object.keys(intents))
    if (!names.has(name)) invalid(name, "unknown field");
}
function entered(field: FieldDescriptor, intent: FieldIntent): WireValue {
  if (intent.mode !== "value" && intent.mode !== "null")
    return invalid(field.name, "invalid field intent");
  return normalizeValue(
    field.shape,
    intent.mode === "value" ? intent.value : null,
    field.name,
  );
}

export function objectInput(
  fields: FieldDescriptor[],
  intents: Record<string, FieldIntent>,
): WireObject {
  known(fields, intents);
  const result: WireObject = Object.create(null);
  for (const field of fields) {
    const intent = Object.hasOwn(intents, field.name)
      ? intents[field.name]
      : ({ mode: "omit" } as const);
    if (intent.mode === "remove")
      invalid(field.name, "remove is only a patch operation");
    if (intent.mode === "omit") {
      if (field.shape.type !== "optional")
        invalid(field.name, "required field");
    } else result[field.name] = entered(field, intent);
  }
  return result;
}

export function patchInput(
  fields: FieldDescriptor[],
  intents: Record<string, FieldIntent>,
): Record<string, FieldUpdate> {
  known(fields, intents);
  const result: Record<string, FieldUpdate> = Object.create(null);
  for (const field of fields) {
    if (!Object.hasOwn(intents, field.name)) continue;
    const intent = intents[field.name];
    if (intent.mode === "omit") continue;
    if (intent.mode === "remove") {
      if (field.shape.type !== "optional")
        invalid(field.name, "only optional fields can be removed");
      result[field.name] = { op: "remove" };
    } else result[field.name] = { op: "set", value: entered(field, intent) };
  }
  return result;
}
