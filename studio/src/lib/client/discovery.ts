import type {
  ActionInput,
  CodecIdentity,
  Discovery,
  FieldDescriptor,
  InputDescriptor,
  Shape,
  WireValue,
} from "./types.ts";
import { record, text, unsigned } from "./validation.ts";
function list(value: WireValue | undefined): WireValue[] {
  if (!Array.isArray(value) || value.length > 1024)
    throw new Error("metadata list limit");
  return value;
}
function unique<T>(values: T[], key: (value: T) => string): T[] {
  if (new Set(values.map(key)).size !== values.length)
    throw new Error("duplicate metadata");
  return values;
}
function version(value: WireValue | undefined): number {
  const n = unsigned(value);
  if (n < 1n || n > 4294967295n) throw new Error("invalid metadata version");
  return Number(n);
}
function codec(value: WireValue | undefined): CodecIdentity | undefined {
  if (value === undefined) return undefined;
  const o = record(value);
  return { name: text(o.name), version: version(o.version) };
}
function shape(value: WireValue, depth = 0): Shape {
  if (depth > 16) throw new Error("shape depth limit");
  const o = record(value);
  const type = text(o.type);
  switch (type) {
    case "string":
    case "bool":
    case "u64":
    case "i64":
    case "f64":
      return { type };
    case "optional":
    case "nullable":
    case "list":
    case "map":
      return { type, value: shape(o.value, depth + 1) };
    case "enum":
      return { type, value: unique(list(o.value).map(text), (v) => v) };
    case "reference":
      return { type, value: { kind: text(record(o.value).kind) } };
    default:
      throw new Error("unsupported shape");
  }
}
function fields(value: WireValue): FieldDescriptor[] {
  return unique(
    list(value).map((v) => {
      const o = record(v);
      return {
        name: text(o.name),
        shape: shape(o.shape),
        ...(o.codec === undefined ? {} : { codec: codec(o.codec) }),
      };
    }),
    (v) => v.name,
  );
}
function input(value: WireValue): InputDescriptor | null {
  if (value === null) return null;
  const o = record(value),
    type = text(o.type);
  if (type === "unit") return { type };
  if (type === "object") return { type, value: fields(o.value) };
  if (type === "scalar") {
    const v = record(o.value);
    return {
      type,
      value: {
        shape: shape(v.shape),
        ...(v.codec === undefined ? {} : { codec: codec(v.codec) }),
      },
    };
  }
  throw new Error("invalid input descriptor");
}
export function discovery(value: WireValue): Discovery {
  const o = record(value);
  if (o.version !== 1) throw new Error("unsupported discovery version");
  const resources = unique(
    list(o.resources).map((v) => {
      const resource = record(v),
        actions = unique(list(resource.actions).map(text), (v) => v);
      const action_inputs: ActionInput[] = unique(
        list(resource.action_inputs).map((v) => {
          const a = record(v);
          return {
            name: text(a.name),
            version: version(a.version),
            input: input(a.input),
          };
        }),
        (v) => v.name,
      );
      if (
        action_inputs.some((a) => !actions.includes(a.name)) ||
        action_inputs.length !== actions.length
      )
        throw new Error("inconsistent action descriptors");
      if (action_inputs.some((a) => a.version !== 1))
        throw new Error("unsupported action input version");
      return {
        kind: text(resource.kind),
        version: version(resource.version),
        fields: fields(resource.fields),
        actions,
        action_inputs,
      };
    }),
    (v) => v.kind,
  );
  return { version: 1, resources };
}
