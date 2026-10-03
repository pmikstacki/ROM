import { parseWire, stringifyWire } from "./codec.ts";
import { record, text, unsigned } from "./validation.ts";
import type {
  ProjectedView,
  QueryAnchor,
  QuerySpec,
  WireObject,
  WireValue,
} from "./types.ts";

function members(
  value: WireValue,
  required: string[],
  optional: string[] = [],
): WireObject {
  const object = record(value);
  if (
    required.some((name) => !Object.hasOwn(object, name)) ||
    Object.keys(object).some(
      (name) => !required.includes(name) && !optional.includes(name),
    )
  )
    throw Error("invalid query anchor members");
  return object;
}

function entries(value: WireValue): WireValue[] {
  if (!Array.isArray(value)) throw Error("invalid query anchor array");
  return value;
}

function predicates(values: WireValue[], comparison: boolean): WireObject[] {
  return values.map((value) => {
    const object = members(
      value,
      comparison ? ["field", "op", "value"] : ["field", "value"],
      ["absent"],
    );
    text(object.field);
    if (object.absent !== undefined && typeof object.absent !== "boolean")
      throw Error("invalid query anchor absence");
    if (
      comparison &&
      (typeof object.op !== "string" ||
        !["eq", "ne", "lt", "le", "gt", "ge"].includes(object.op))
    )
      throw Error("invalid query anchor comparison");
    if (
      object.absent === true &&
      (object.value !== null ||
        (comparison && object.op !== "eq" && object.op !== "ne"))
    )
      throw Error("invalid query anchor absence predicate");
    return object;
  });
}

function orders(values: WireValue[]): WireObject[] {
  const result = values.map((value) => {
    const object = members(value, ["field", "direction"]);
    text(object.field);
    if (object.direction !== "asc" && object.direction !== "desc")
      throw Error("invalid query anchor direction");
    return object;
  });
  if (new Set(result.map((entry) => entry.field)).size !== result.length)
    throw Error("duplicate query anchor order");
  return result;
}

function corresponds(
  actual: WireObject[],
  expected: WireValue[],
  fields: string[],
): void {
  if (
    actual.length !== expected.length ||
    actual.some((item, index) => {
      const requested = record(expected[index]);
      return fields.some((name) =>
        name === "absent"
          ? (item.absent ?? false) !== (requested.absent ?? false)
          : item[name] !== requested[name],
      );
    })
  )
    throw Error("query anchor request mismatch");
}

export function acceptedAnchor(
  value: WireValue,
  kind: string,
  id: string,
  query?: QuerySpec,
): QueryAnchor {
  try {
    const o = members(value, [
      "version",
      "kind",
      "schema_version",
      "filters",
      "comparisons",
      "order",
      "id",
      "values",
    ]);
    if (
      unsigned(o.version, o, "version") !== 1n ||
      text(o.kind) !== kind ||
      text(o.id) !== id
    )
      throw Error("query anchor identity mismatch");
    const schema = unsigned(o.schema_version, o, "schema_version");
    if (schema < 1n || schema > 4294967295n)
      throw Error("query anchor schema version");
    const filters = entries(o.filters),
      comparisons = entries(o.comparisons),
      order = entries(o.order),
      values = entries(o.values);
    if (
      order.length > 4 ||
      values.length !== order.length ||
      filters.length + comparisons.length > 32
    )
      throw Error("query anchor limit");
    const admittedFilters = predicates(filters, false),
      admittedComparisons = predicates(comparisons, true),
      admittedOrder = orders(order);
    for (const value of values) {
      const object = record(value);
      if (object.state === "missing") members(object, ["state"]);
      else if (object.state === "value") members(object, ["state", "value"]);
      else throw Error("invalid query anchor state");
    }
    if (query) {
      corresponds(admittedFilters, (query.filters ?? []) as WireValue[], [
        "field",
        "absent",
      ]);
      corresponds(
        admittedComparisons,
        (query.comparisons ?? []) as WireValue[],
        ["field", "op", "absent"],
      );
      corresponds(admittedOrder, (query.order ?? []) as WireValue[], [
        "field",
        "direction",
      ]);
    }
    return {
      kind,
      id,
      schema_version: Number(schema),
      canonical: stringifyWire(value),
    };
  } catch (error) {
    throw Error(
      `invalid query anchor: ${error instanceof Error ? error.message : "invalid envelope"}`,
    );
  }
}
export function queryWire(kind: string, query: QuerySpec): WireObject {
  const { after, ...base } = query;
  if (after === undefined || after === null)
    return {
      ...base,
      ...(after === null ? { after: null } : {}),
    } as unknown as WireObject;
  if (typeof after.canonical !== "string") throw Error("invalid query anchor");
  const parsed = parseWire(after.canonical),
    accepted = acceptedAnchor(parsed, kind, after.id, query);
  if (accepted.schema_version !== after.schema_version || after.kind !== kind)
    throw Error("query anchor identity mismatch");
  return { ...base, after: parsed } as unknown as WireObject;
}
export function anchorRequest(
  query: QuerySpec,
  last: ProjectedView,
): WireObject {
  if (last.value === null) throw Error("deleted view has no query anchor");
  const { after: _after, after_id: _id, ...spec } = query;
  const value = record(parseWire(stringifyWire(last.value)));
  const required = new Set((query.order ?? []).map((o) => o.field));
  for (const key of Object.keys(value))
    if (!required.has(key)) delete value[key];
  return {
    query: spec,
    view: { key: last.key, revision: last.revision, value },
  } as unknown as WireObject;
}

/** The requested page and client admission policy both bound disclosure. */
export function pageLimit(query: QuerySpec, maxRows: number): number {
  if (query.limit === undefined || query.limit === null) return maxRows;
  if (!Number.isSafeInteger(query.limit) || query.limit < 1)
    throw Error("invalid query limit");
  return Math.min(query.limit, maxRows);
}
