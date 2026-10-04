import type {
  CompareOp,
  FieldDescriptor,
  QuerySpec,
  ResourceDescriptor,
  Shape,
} from "../client/types.ts";
import { normalizeValue } from "../client/codec.ts";
import { baseShape } from "../renderers/default-value.ts";
import { serializeBranch } from "./vendor/composition.ts";
import type { FilterDraft, FilterRule, FilterSet } from "./types.ts";
const operators: Record<string, CompareOp> = {
  equal: "eq",
  notEqual: "ne",
  less: "lt",
  lessOrEqual: "le",
  greater: "gt",
  greaterOrEqual: "ge",
};
export function scalarShape(shape: Shape): boolean {
  return !["list", "map"].includes(baseShape(shape).type);
}
export function filterOperators(
  field: FieldDescriptor,
): { value: string; label: string }[] {
  const options = [
    { value: "equal", label: "Equals" },
    { value: "notEqual", label: "Does not equal" },
  ];
  if (scalarShape(field.shape))
    options.push(
      { value: "less", label: "Less than" },
      { value: "lessOrEqual", label: "At most" },
      { value: "greater", label: "Greater than" },
      { value: "greaterOrEqual", label: "At least" },
    );
  if (field.shape.type === "optional")
    options.push(
      { value: "absent", label: "Is absent" },
      { value: "present", label: "Is present" },
    );
  return options;
}
function field(descriptor: ResourceDescriptor, name: string) {
  const result = descriptor.fields.find((f) => f.name === name);
  if (!result) throw Error(`Unknown filter field: ${name}`);
  return result;
}
function keys(value: object, allowed: string[]) {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw Error("Invalid filter: plain object required.");
  if (![Object.prototype, null].includes(Object.getPrototypeOf(value)))
    throw Error("Invalid filter: plain object required.");
  for (const key of Reflect.ownKeys(value)) {
    if (typeof key !== "string") throw Error("Filter symbols are unsupported.");
    if (!allowed.includes(key))
      throw Error(`unsupported filter attribute: ${key}`);
    const property = Object.getOwnPropertyDescriptor(value, key);
    if (!property || !("value" in property))
      throw Error("Filter getters are unsupported.");
  }
}
export function conjunctionRules(set: FilterSet, depth = 0): FilterRule[] {
  if (depth > 8) throw Error("Filter group depth limit reached.");
  if (!set || typeof set !== "object" || Array.isArray(set))
    throw Error("Invalid filter group.");
  keys(set, ["glue", "rules"]);
  if (set.glue !== "and") throw Error("Only AND filter groups are supported.");
  if (!Array.isArray(set.rules) || set.rules.length > 32)
    throw Error("At most 32 filter rules are supported.");
  const out: FilterRule[] = [];
  for (const item of set.rules) {
    if (!item || typeof item !== "object" || Array.isArray(item))
      throw Error("Invalid filter rule.");
    if ("rules" in item) out.push(...conjunctionRules(item, depth + 1));
    else {
      keys(item, ["field", "filter", "value"]);
      if (
        typeof item.field !== "string" ||
        typeof item.filter !== "string" ||
        !Object.hasOwn(item, "value")
      )
        throw Error("Invalid filter rule.");
      out.push(item);
    }
    if (out.length > 32) throw Error("At most 32 filter rules are supported.");
  }
  return out;
}
export function queryFromDraft(
  descriptor: ResourceDescriptor,
  draft: FilterDraft,
): QuerySpec {
  keys(draft, ["rules", "order", "limit"]);
  const source = conjunctionRules(draft.rules); // Validate before the vendored serializer; it must not hide unknown attributes.
  const group = serializeBranch({ glue: "and", rules: source }) as FilterSet;
  if (typeof draft.limit !== "string" || !/^\d+$/.test(draft.limit))
    throw Error("Limit must be between 1 and 1000.");
  const limit = Number(draft.limit);
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > 1000)
    throw Error("Limit must be between 1 and 1000.");
  const result: QuerySpec = { filters: [], comparisons: [], order: [], limit };
  for (const item of group.rules as FilterRule[]) {
    const f = field(descriptor, item.field);
    if (item.filter === "absent" || item.filter === "present") {
      if (f.shape.type !== "optional")
        throw Error("Absence requires an optional field.");
      if (item.value !== null) throw Error("Absence requires a null sentinel.");
      if (item.filter === "absent")
        result.filters!.push({ field: f.name, value: null, absent: true });
      else
        result.comparisons!.push({
          field: f.name,
          op: "ne",
          value: null,
          absent: true,
        });
      continue;
    }
    const op = Object.hasOwn(operators, item.filter)
      ? operators[item.filter]
      : undefined;
    if (!op) throw Error(`unsupported filter operator: ${item.filter}`);
    const value = normalizeValue(f.shape, item.value, f.name);
    if (op !== "eq" && op !== "ne" && (!scalarShape(f.shape) || value === null))
      throw Error("Ordered comparisons require a non-null scalar value.");
    if (op === "eq") result.filters!.push({ field: f.name, value });
    else result.comparisons!.push({ field: f.name, op, value });
  }
  const order = draft.order ?? [];
  if (!Array.isArray(order) || order.length > 4)
    throw Error("At most 4 order fields are supported.");
  const seen = new Set<string>();
  for (const item of order) {
    keys(item, ["field", "direction"]);
    const f = field(descriptor, item.field);
    if (!scalarShape(f.shape)) throw Error("Sort requires a scalar field.");
    if (seen.has(item.field)) throw Error("duplicate sort field");
    seen.add(item.field);
    if (item.direction !== "asc" && item.direction !== "desc")
      throw Error("Invalid sort direction.");
    result.order!.push({ ...item });
  }
  return result;
}
export function draftFromQuery(
  descriptor: ResourceDescriptor,
  query: QuerySpec,
): FilterDraft {
  const converted: FilterRule[] = [];
  for (const f of query.filters ?? [])
    converted.push({
      field: f.field,
      filter: f.absent ? "absent" : "equal",
      value: f.value,
    });
  for (const c of query.comparisons ?? []) {
    const filter = Object.entries(operators).find(([, op]) => op === c.op)?.[0];
    if (!filter) throw Error("Unsupported saved query operator.");
    if (c.absent && c.op !== "eq" && c.op !== "ne")
      throw Error("Unsupported absence operator.");
    converted.push({
      field: c.field,
      filter: c.absent ? (c.op === "eq" ? "absent" : "present") : filter,
      value: c.value,
    });
  }
  const draft: FilterDraft = {
    rules: { glue: "and", rules: converted },
    order: (query.order ?? []).map((o) => ({ ...o })),
    limit: String(query.limit ?? 50),
  };
  const validated = queryFromDraft(descriptor, draft); // Clone nested values through ROM's codec, not JSON serialization.
  const output = draft.rules.rules as FilterRule[];
  let f = 0,
    c = 0;
  for (const item of output) {
    if (item.filter === "equal" || item.filter === "absent")
      item.value = validated.filters![f++].value;
    else item.value = validated.comparisons![c++].value;
  }
  return draft;
}
