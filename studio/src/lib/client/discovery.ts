import type {
  ActionInput,
  CodecWrapper,
  WireObject,
  ResourcePresentation,
  FieldPresentation,
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
function version(owner: WireObject): number {
  const n = unsigned(owner.version, owner, "version");
  if (n < 1n || n > 4294967295n) throw new Error("invalid metadata version");
  return Number(n);
}
function codec(value: WireValue | undefined): CodecIdentity | undefined {
  if (value === undefined) return undefined;
  const o = record(value);
  const name = text(o.name);
  if (new TextEncoder().encode(name).byteLength > 256)
    throw Error("codec identity bytes limit");
  return { name, version: version(o) };
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
      if (depth !== 0) throw Error("nested optional shape");
      return { type, value: shape(o.value, depth + 1) };
    case "nullable": {
      const inner = shape(o.value, depth + 1);
      if (inner.type === "nullable") throw Error("nested nullable shape");
      return { type, value: inner };
    }
    case "list":
    case "map":
      return { type, value: shape(o.value, depth + 1) };
    case "enum": {
      const values = list(o.value);
      if (values.length < 1 || values.length > 256)
        throw Error("enum shape limit");
      return { type, value: unique(values.map(text), (v) => v) };
    }
    case "reference":
      return { type, value: { kind: text(record(o.value).kind) } };
    default:
      throw new Error("unsupported shape");
  }
}
function wrappers(o: WireObject, shape: Shape): CodecWrapper[] {
  const path = o.codec_wrappers === undefined ? [] : o.codec_wrappers;
  if (
    !Array.isArray(path) ||
    path.length > 16 ||
    (o.codec === undefined && path.length)
  )
    throw Error("invalid codec wrapper path");
  let current = shape;
  for (const wrapper of path) {
    if (
      !["optional", "nullable", "list", "map"].includes(String(wrapper)) ||
      current.type !== wrapper ||
      !("value" in current)
    )
      throw Error("invalid codec wrapper path");
    current = current.value as Shape;
  }
  return path as CodecWrapper[];
}
function fields(value: WireValue): FieldDescriptor[] {
  return unique(
    list(value).map((v) => {
      const o = record(v),
        fieldShape = shape(o.shape),
        path = wrappers(o, fieldShape);
      return {
        name: text(o.name),
        shape: fieldShape,
        ...(path.length ? { codec_wrappers: path } : {}),
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
    const v = record(o.value),
      fieldShape = shape(v.shape),
      path = wrappers(v, fieldShape);
    if (fieldShape.type === "optional") throw Error("scalar presence shape");
    return {
      type,
      value: {
        shape: fieldShape,
        ...(path.length ? { codec_wrappers: path } : {}),
        ...(v.codec === undefined ? {} : { codec: codec(v.codec) }),
      },
    };
  }
  throw new Error("invalid input descriptor");
}
function presentation(
  value: WireValue,
  fields: FieldDescriptor[],
): ResourcePresentation {
  const p = record(value);
  const checkKeys = (o: WireObject, allowed: string[]) => {
    if (Object.keys(o).some((key) => !allowed.includes(key)))
      throw Error("invalid presentation property");
  };
  const bounded = (value: WireValue | undefined, bytes = 256): string => {
    if (
      typeof value !== "string" ||
      value.length === 0 ||
      new TextEncoder().encode(value).byteLength > bytes
    )
      throw Error("invalid presentation text");
    return value;
  };
  checkKeys(p, ["label", "title_field", "fields", "groups", "settings"]);
  const result: ResourcePresentation = {};
  if (p.label !== undefined) result.label = bounded(p.label);
  if (p.groups !== undefined) {
    result.groups = list(p.groups).map((value) => {
      const group = record(value);
      checkKeys(group, ["name", "label"]);
      return { name: bounded(group.name), label: bounded(group.label) };
    });
    if (new Set(result.groups.map((g) => g.name)).size !== result.groups.length)
      throw Error("duplicate presentation group");
  }
  if (p.fields !== undefined) {
    const hints = record(p.fields);
    if (Object.keys(hints).length > 1024)
      throw Error("presentation fields limit");
    result.fields = Object.create(null) as Record<string, FieldPresentation>;
    for (const [name, value] of Object.entries(hints)) {
      if (!fields.some((field) => field.name === name))
        throw Error("unknown presentation field");
      const hint = record(value);
      checkKeys(hint, ["label", "help", "group"]);
      const field: FieldPresentation = {};
      if (hint.label !== undefined) field.label = bounded(hint.label);
      if (hint.help !== undefined) field.help = bounded(hint.help, 2048);
      if (hint.group !== undefined) {
        field.group = bounded(hint.group);
        if (!result.groups?.some((g) => g.name === field.group))
          throw Error("unknown presentation group");
      }
      result.fields[name] = field;
    }
  }
  if (p.title_field !== undefined) {
    result.title_field = bounded(p.title_field);
    const field = fields.find((field) => field.name === result.title_field);
    let shape = field?.shape;
    while (shape?.type === "optional" || shape?.type === "nullable")
      shape = shape.value;
    if (!field || field.codec || shape?.type !== "string")
      throw Error("invalid presentation title field");
  }
  if (p.settings !== undefined) {
    const settings = record(p.settings);
    checkKeys(settings, ["group", "label"]);
    result.settings = {
      group: bounded(settings.group),
      label: bounded(settings.label),
    };
  }
  return result;
}
export function discovery(value: WireValue): Discovery {
  const o = record(value);
  if (unsigned(o.version, o, "version") !== 1n)
    throw new Error("unsupported discovery version");
  const resources = unique(
    list(o.resources).map((v) => {
      const resource = record(v),
        actions = unique(list(resource.actions).map(text), (v) => v);
      const action_inputs: ActionInput[] = unique(
        list(resource.action_inputs).map((v) => {
          const a = record(v);
          return {
            name: text(a.name),
            version: version(a),
            input: input(a.input),
          };
        }),
        (v) => v.name,
      );
      if (action_inputs.some((a) => !actions.includes(a.name)))
        throw new Error("inconsistent action descriptors");
      if (action_inputs.some((a) => a.version !== 1))
        throw new Error("unsupported action input version");
      const resourceFields = fields(resource.fields);
      return {
        kind: text(resource.kind),
        version: version(resource),
        fields: resourceFields,
        ...(resource.presentation === undefined
          ? {}
          : {
              presentation: presentation(resource.presentation, resourceFields),
            }),
        actions,
        action_inputs,
      };
    }),
    (v) => v.kind,
  );
  return { version: 1, resources };
}
