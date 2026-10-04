import assert from "node:assert/strict";
import test from "node:test";
import {
  draftFromQuery,
  queryFromDraft,
} from "../../src/lib/filters/translation.ts";
import type { FilterDraft } from "../../src/lib/filters/types.ts";
import type {
  QuerySpec,
  ResourceDescriptor,
} from "../../src/lib/client/types.ts";
const descriptor: ResourceDescriptor = {
  kind: "held-out",
  version: 1,
  actions: [],
  action_inputs: [],
  fields: [
    { name: "title", shape: { type: "string" } },
    { name: "enabled", shape: { type: "bool" } },
    { name: "count", shape: { type: "u64" } },
    { name: "signed", shape: { type: "i64" } },
    { name: "score", shape: { type: "f64" } },
    {
      name: "note",
      shape: {
        type: "optional",
        value: { type: "nullable", value: { type: "string" } },
      },
    },
    {
      name: "code",
      shape: { type: "string" },
      codec: { name: "author-code", version: 1 },
    },
    { name: "items", shape: { type: "list", value: { type: "string" } } },
  ],
};
const draft = (rules: FilterDraft["rules"]["rules"]): FilterDraft => ({
  rules: { glue: "and", rules },
  order: [],
  limit: "50",
});
test("filter round trip preserves exact values, comparisons and ordered fields", () => {
  const query: QuerySpec = {
    filters: [
      { field: "enabled", value: false },
      { field: "title", value: "" },
      { field: "count", value: 18446744073709551615n },
      { field: "signed", value: -9223372036854775808n },
      { field: "score", value: 0 },
      { field: "note", value: null },
      { field: "note", value: null, absent: true },
      { field: "code", value: "AUTHOR-A" },
    ],
    comparisons: [{ field: "score", op: "ge", value: 0 }],
    order: [{ field: "count", direction: "desc" }],
    limit: 50,
  };
  assert.deepEqual(
    queryFromDraft(descriptor, draftFromQuery(descriptor, query)),
    query,
  );
});
test("nested conjunctions retain all rules without invoking a numeric parser", () => {
  assert.deepEqual(
    queryFromDraft(
      descriptor,
      draft([
        {
          glue: "and",
          rules: [
            { field: "count", filter: "greater", value: 9007199254740993n },
            { field: "title", filter: "equal", value: "" },
          ],
        },
      ]),
    ),
    {
      filters: [{ field: "title", value: "" }],
      comparisons: [{ field: "count", op: "gt", value: 9007199254740993n }],
      order: [],
      limit: 50,
    },
  );
});
test("unsupported operators, OR and hidden fields are rejected without silent dropping", () => {
  for (const bad of [
    { field: "title", filter: "contains", value: "a" },
    { field: "hidden", filter: "equal", value: "a" },
    { glue: "or", rules: [{ field: "title", filter: "equal", value: "a" }] },
  ])
    assert.throws(
      () => queryFromDraft(descriptor, draft([bad])),
      /unsupported|Unknown|AND/,
    );
});
test("absence is distinct from null and allowed only for optional fields", () => {
  assert.deepEqual(
    queryFromDraft(
      descriptor,
      draft([{ field: "note", filter: "absent", value: null }]),
    ).filters,
    [{ field: "note", value: null, absent: true }],
  );
  assert.throws(
    () =>
      queryFromDraft(
        descriptor,
        draft([{ field: "title", filter: "absent", value: null }]),
      ),
    /optional/,
  );
  assert.throws(
    () =>
      queryFromDraft(
        descriptor,
        draft([{ field: "note", filter: "absent", value: "" }]),
      ),
    /null/,
  );
  assert.throws(
    () =>
      queryFromDraft(
        descriptor,
        draft([{ field: "title", filter: "equal", value: null }]),
      ),
    /string/,
  );
});
test("native query bounds and scalar comparison restrictions apply before execution", () => {
  assert.throws(
    () =>
      queryFromDraft(
        descriptor,
        draft(
          Array.from({ length: 33 }, () => ({
            field: "title",
            filter: "equal",
            value: "a",
          })),
        ),
      ),
    /32/,
  );
  assert.throws(
    () =>
      queryFromDraft(descriptor, {
        ...draft([]),
        order: Array.from({ length: 5 }, () => ({
          field: "title",
          direction: "asc",
        })),
      }),
    /4/,
  );
  assert.throws(
    () =>
      queryFromDraft(descriptor, {
        ...draft([]),
        order: [
          { field: "title", direction: "asc" },
          { field: "title", direction: "desc" },
        ],
      }),
    /duplicate/,
  );
  assert.throws(
    () =>
      queryFromDraft(
        descriptor,
        draft([{ field: "items", filter: "less", value: [] }]),
      ),
    /scalar/,
  );
  assert.throws(
    () =>
      queryFromDraft(
        descriptor,
        draft([{ field: "note", filter: "greater", value: null }]),
      ),
    /null/,
  );
  for (const limit of ["0", "1001", "1.2", "", "2x"])
    assert.throws(
      () => queryFromDraft(descriptor, { ...draft([]), limit }),
      /Limit/,
    );
});
test("malformed trees, cyclic groups and unknown attributes are rejected", () => {
  const cyclic: FilterDraft = draft([]);
  cyclic.rules.rules.push(cyclic.rules);
  assert.throws(() => queryFromDraft(descriptor, cyclic), /depth|cyclic/);
  const bad = Object.assign(
    { field: "title", filter: "equal", value: "" },
    { includes: ["a"] },
  );
  assert.throws(() => queryFromDraft(descriptor, draft([bad])), /unsupported/);
});
test("draft conversion resets page continuation and never aliases active query arrays", () => {
  const q: QuerySpec = {
    filters: [{ field: "title", value: "first" }],
    order: [{ field: "title", direction: "asc" }],
    limit: 20,
    after_id: "old-page",
  };
  const d = draftFromQuery(descriptor, q);
  d.order![0].direction = "desc";
  const r = d.rules.rules[0];
  if ("field" in r) r.value = "second";
  assert.deepEqual(q.filters, [{ field: "title", value: "first" }]);
  assert.equal(q.order![0].direction, "asc");
  const next = queryFromDraft(descriptor, d);
  assert.equal(next.after_id, undefined);
});
test("invalid saved query operators cannot enter an editable draft", () => {
  const q: QuerySpec = {
    comparisons: [{ field: "title", op: "wat" as "eq", value: "a" }],
  };
  assert.throws(() => draftFromQuery(descriptor, q), /operator/);
});
test("wrapped custom values are cloned without aliasing or loss of reserved map names", () => {
  const wrapped: ResourceDescriptor = {
    ...descriptor,
    fields: [
      {
        name: "metadata",
        shape: { type: "map", value: { type: "u64" } },
        codec: { name: "counter", version: 1 },
        codec_wrappers: ["map"],
      },
    ],
  };
  const value = Object.create(null);
  value.__proto__ = 18446744073709551615n;
  const q: QuerySpec = { filters: [{ field: "metadata", value }] };
  const d = draftFromQuery(wrapped, q);
  const item = d.rules.rules[0];
  assert.ok("field" in item);
  assert.ok(
    item.value && typeof item.value === "object" && !Array.isArray(item.value),
  );
  item.value.__proto__ = 0n;
  assert.equal(value.__proto__, 18446744073709551615n);
  assert.equal(
    queryFromDraft(wrapped, d).filters![0].value &&
      Object.getPrototypeOf(queryFromDraft(wrapped, d).filters![0].value),
    null,
  );
});
test("optional present comparison survives saved query conversion", () => {
  const q: QuerySpec = {
    comparisons: [{ field: "note", op: "ne", value: null, absent: true }],
    limit: 50,
  };
  const next = queryFromDraft(descriptor, draftFromQuery(descriptor, q));
  assert.deepEqual(next.comparisons, q.comparisons);
});
test("inherited fields and getter rules are rejected without evaluating the getter", () => {
  let calls = 0;
  const rule = {
    field: "title",
    filter: "equal",
    get value() {
      calls++;
      return "secret";
    },
  };
  assert.throws(() => queryFromDraft(descriptor, draft([rule])), /getters/);
  assert.equal(calls, 0);
  const inherited = Object.assign(
    Object.create({ field: "title", filter: "equal" }),
    { value: "a" },
  );
  assert.throws(
    () => queryFromDraft(descriptor, draft([inherited])),
    /plain|Invalid/,
  );
});

test("non-enumerable filter accessors are rejected before invocation", () => {
  let calls = 0;
  const rule = { field: "title", filter: "equal" };
  Object.defineProperty(rule, "value", {
    get() {
      calls++;
      return "secret";
    },
  });
  assert.throws(
    () =>
      queryFromDraft(
        descriptor,
        draft([
          rule as unknown as import("../../src/lib/filters/types.ts").FilterRule,
        ]),
      ),
    /getters|accessor/,
  );
  assert.equal(calls, 0);
  const group = { glue: "and" };
  Object.defineProperty(group, "rules", {
    get() {
      calls++;
      return [];
    },
  });
  assert.throws(
    () =>
      queryFromDraft(descriptor, {
        ...draft([]),
        rules: group as unknown as FilterDraft["rules"],
      }),
    /getters|accessor/,
  );
  assert.equal(calls, 0);
});

test("draft envelope rejects inherited members, symbols, unknown and hidden getters", () => {
  let calls = 0;
  const hidden = { rules: draft([]).rules, order: [] };
  Object.defineProperty(hidden, "limit", {
    get() {
      calls++;
      return "50";
    },
  });
  assert.throws(
    () => queryFromDraft(descriptor, hidden as unknown as FilterDraft),
    /getters|accessor/,
  );
  assert.equal(calls, 0);
  for (const bad of [
    Object.assign(Object.create({ limit: "50" }), { rules: draft([]).rules }),
    Object.assign(draft([]), { extra: true }),
    Object.assign(draft([]), { [Symbol("secret")]: true }),
  ]) {
    assert.throws(
      () => queryFromDraft(descriptor, bad),
      /plain|unsupported|symbol/,
    );
  }
  assert.equal(
    queryFromDraft(descriptor, {
      rules: { glue: "and", rules: [] },
      limit: "50",
    }).limit,
    50,
  );
});
