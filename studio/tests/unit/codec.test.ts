import assert from "node:assert/strict";
import test from "node:test";
import {
  parseWire,
  stringifyWire,
  normalizeValue,
  objectInput,
  patchInput,
} from "../../src/lib/client/codec.ts";
import type {
  FieldDescriptor,
  WireObject,
} from "../../src/lib/client/types.ts";

test("wire codec preserves boundary integers and ordinary false/zero/null/empty values", () => {
  const text =
    '{"u":18446744073709551615,"i":-9223372036854775808,"zero":0,"no":false,"nil":null,"empty":""}';
  const result = parseWire(text);
  assert.equal(typeof result, "object");
  assert.ok(result && !Array.isArray(result));
  assert.equal(result.u, 18446744073709551615n);
  assert.equal(result.i, -9223372036854775808n);
  assert.equal(result.zero, 0);
  assert.equal(result.no, false);
  assert.equal(result.nil, null);
  assert.equal(result.empty, "");
  assert.equal(stringifyWire(result), text);
});

test("wire parser rejects duplicate keys, oversized text, excessive nesting, and infinite numbers", () => {
  assert.throws(() => parseWire('{"value":1,"value":2}'));
  assert.throws(() => parseWire('"éé"', 5), /limit/);
  assert.throws(() => parseWire("[[[[[0]]]]]", 1024, 4), /depth/);
  assert.throws(() => parseWire("1e9999"), /finite/);
  assert.throws(() => parseWire("1" + "0".repeat(300)), /number/);
});

test("wire maps preserve reserved names without prototype mutation", () => {
  const result = parseWire(
    '{"__proto__":{"polluted":true},"constructor":"a value"}',
  );
  assert.ok(result && typeof result === "object" && !Array.isArray(result));
  assert.equal(Object.getPrototypeOf(result), null);
  assert.equal(Object.hasOwn(result, "__proto__"), true);
  assert.equal(result.constructor, "a value");
  assert.equal(Object.hasOwn({}, "polluted"), false);
  assert.equal(
    stringifyWire(result),
    '{"__proto__":{"polluted":true},"constructor":"a value"}',
  );
});

test("wire encoder refuses nonfinite and recursive values before sending them", () => {
  assert.throws(() => stringifyWire(NaN), /finite/);
  assert.throws(() => stringifyWire(Infinity), /finite/);
  const loop: { [key: string]: null | typeof loop } = {};
  loop.loop = loop;
  assert.throws(() => stringifyWire(loop), /cyclic/);
  assert.throws(() => stringifyWire("éé", 5), /limit/);
});

test("scalar normalization applies exact integer ranges and finite float semantics", () => {
  assert.equal(
    normalizeValue({ type: "u64" }, 18446744073709551615n),
    18446744073709551615n,
  );
  assert.equal(
    normalizeValue({ type: "i64" }, -9223372036854775808n),
    -9223372036854775808n,
  );
  assert.equal(normalizeValue({ type: "u64" }, 0), 0n);
  assert.throws(() => normalizeValue({ type: "u64" }, -1n));
  assert.throws(() => normalizeValue({ type: "u64" }, 18446744073709551616n));
  assert.throws(() => normalizeValue({ type: "i64" }, -9223372036854775809n));
  assert.throws(
    () => normalizeValue({ type: "u64" }, Number.MAX_SAFE_INTEGER + 1),
    /safe/,
  );
  assert.equal(normalizeValue({ type: "f64" }, 1e20), 1e20);
  assert.throws(() => normalizeValue({ type: "f64" }, Infinity));
  assert.equal(normalizeValue({ type: "bool" }, false), false);
});

const fields: FieldDescriptor[] = [
  { name: "title", shape: { type: "string" } },
  { name: "done", shape: { type: "bool" } },
  {
    name: "note",
    shape: {
      type: "optional",
      value: { type: "nullable", value: { type: "string" } },
    },
  },
];

test("object and patch encoders distinguish omission, null, value and removal", () => {
  const created = objectInput(fields, {
    title: { mode: "value", value: "" },
    done: { mode: "value", value: false },
    note: { mode: "omit" },
  });
  assert.equal(created.title, "");
  assert.equal(created.done, false);
  assert.equal(Object.hasOwn(created, "note"), false);
  const patch = patchInput(fields, {
    note: { mode: "null" },
    done: { mode: "value", value: false },
    title: { mode: "omit" },
  });
  assert.equal(patch.note.op, "set");
  assert.equal(patch.note.op === "set" && patch.note.value, null);
  assert.equal(Object.hasOwn(patch, "title"), false);
  assert.equal(
    patchInput(fields, { note: { mode: "remove" } }).note.op,
    "remove",
  );
  assert.throws(
    () =>
      objectInput(fields, {
        title: { mode: "omit" },
        done: { mode: "value", value: false },
      }),
    /required/,
  );
  assert.throws(
    () =>
      objectInput(fields, {
        title: { mode: "value", value: "a" },
        done: { mode: "value", value: false },
        note: { mode: "remove" },
      }),
    /remove/,
  );
  assert.throws(
    () => patchInput(fields, { done: { mode: "remove" } }),
    /optional/,
  );
  assert.throws(
    () => patchInput(fields, { unknown: { mode: "value", value: true } }),
    /unknown/,
  );
});

test("structural normalization bounds nesting and validates enums, references and maps", () => {
  assert.equal(
    normalizeValue({ type: "enum", value: ["open", "closed"] }, "open"),
    "open",
  );
  assert.throws(() =>
    normalizeValue({ type: "enum", value: ["open"] }, "other"),
  );
  assert.throws(() =>
    normalizeValue({ type: "reference", value: { kind: "tasks" } }, ""),
  );
  const value = normalizeValue(
    { type: "map", value: { type: "list", value: { type: "i64" } } },
    { first: [1, -2n] },
  );
  assert.equal(stringifyWire(value), '{"first":[1,-2]}');
});

test("malformed runtime modes and shapes are refused, not coerced", () => {
  assert.throws(
    () =>
      objectInput(
        [
          {
            name: "flag",
            shape: { type: "nullable", value: { type: "bool" } },
          },
        ],
        { flag: { mode: "mistyped" } } as never,
      ),
    /intent/,
  );
  assert.throws(
    () => normalizeValue({ type: "new-shape" } as never, true),
    /shape/,
  );
});
test("wire strings agree with Rust Unicode and preserve signed zero", () => {
  assert.equal(Object.is(parseWire("-0"), -0), true);
  assert.throws(() => parseWire('"\\ud800"'), /Unicode/);
  assert.throws(() => stringifyWire("\udc00"), /Unicode/);
  assert.equal(parseWire('"\\ud83d\\ude00"'), "😀");
});
test("array and map accessors are rejected without executing user code", () => {
  let reads = 0;
  const array = [1];
  Object.defineProperty(array, "0", {
    get() {
      reads++;
      return 1;
    },
    enumerable: true,
  });
  assert.throws(() => stringifyWire(array), /getter/);
  assert.equal(reads, 0);
  const map = {};
  Object.defineProperty(map, "x", {
    get() {
      reads++;
      return true;
    },
    enumerable: true,
  });
  assert.throws(
    () => normalizeValue({ type: "map", value: { type: "bool" } }, map),
    /getter/,
  );
  assert.equal(reads, 0);
});

test("parsed floating integer tokens retain their Rust number category on replay", () => {
  const input = '{"one":1.0,"exponent":1e3,"items":[2.0,-0.0]}';
  const value = parseWire(input);
  assert.equal(stringifyWire(value), input);
  const map = value as WireObject;
  map.one = 2;
  assert.match(stringifyWire(map), /"one":2,/);
});
