import test from "node:test";
import assert from "node:assert/strict";
import { validateLayout } from "../../src/lib/ui/layout.ts";
const options = {
  columns: 12,
  maxRows: 20,
  maxItems: 3,
  catalog: [
    { id: "history", minWidth: 2, maxWidth: 12, minHeight: 1, maxHeight: 8 },
    { id: "summary", minWidth: 1, maxWidth: 6, minHeight: 1, maxHeight: 4 },
  ],
};
const item = { id: "history", x: 0, y: 0, width: 4, height: 3, visible: true };

test("layout capture detaches and freezes only a valid host-selected catalog", () => {
  const input = [{ ...item }];
  const result = validateLayout(input, options);
  assert.equal(result.valid, true);
  if (!result.valid) return;
  input[0].width = 12;
  assert.equal(result.layout[0].width, 4);
  assert.equal(Object.isFrozen(result.layout), true);
  assert.equal(Object.isFrozen(result.layout[0]), true);
});

test("unknown duplicate and malformed layout IDs are rejected without a fallback", () => {
  for (const input of [
    [item, item],
    [{ ...item, id: "unknown" }],
    [{ ...item, id: "" }],
  ]) {
    assert.equal(validateLayout(input, options).valid, false);
  }
});

test("position size and visible values must satisfy finite integer catalog bounds", () => {
  for (const changed of [
    { x: -1 },
    { y: 19 },
    { width: 1 },
    { width: 13 },
    { height: 9 },
    { x: 10 },
    { height: NaN },
    { width: 2.5 },
    { visible: "true" },
  ])
    assert.equal(
      validateLayout([{ ...item, ...changed }], options).valid,
      false,
    );
  assert.equal(
    validateLayout([{ ...item, visible: false }], options).valid,
    true,
  );
});

test("corrupt stored layouts and entry limits are rejected without throwing", () => {
  for (const input of [
    null,
    {},
    "bad",
    [null],
    [{ ...item, extra: "unexpected" }],
    Array(4).fill(item),
  ]) {
    assert.equal(validateLayout(input, options).valid, false);
  }
  const getter = {
    ...item,
    get width() {
      throw new Error("corrupt stored value");
    },
  };
  assert.equal(validateLayout([getter], options).valid, false);
});

test("invalid host catalogs and limits are configuration errors", () => {
  for (const changed of [
    { columns: 0 },
    { maxRows: Infinity },
    { maxItems: -1 },
    { catalog: [options.catalog[0], options.catalog[0]] },
    { catalog: [{ ...options.catalog[0], minWidth: 13 }] },
  ]) {
    assert.throws(
      () => validateLayout([item], { ...options, ...changed }),
      TypeError,
    );
  }
});

test("stored getters cannot widen the captured host bounds", () => {
  const host = { ...options };
  const input = {
    ...item,
    x: 11,
    get width() {
      host.columns = 100;
      return 4;
    },
  };
  assert.equal(validateLayout([input], host).valid, false);
});

test("inherited and changing catalog bounds remain in the exact validated snapshot", () => {
  const inherited = Object.create({
    minWidth: 1,
    maxWidth: 6,
    minHeight: 1,
    maxHeight: 4,
  });
  inherited.id = "history";
  assert.equal(
    validateLayout([{ ...item, width: 10, height: 10 }], {
      ...options,
      catalog: [inherited],
    }).valid,
    false,
  );
  let reads = 0;
  const changing = {
    ...options.catalog[1],
    id: "history",
    get maxWidth() {
      return ++reads <= 2 ? 6 : 12;
    },
  };
  assert.equal(
    validateLayout([{ ...item, width: 10 }], {
      ...options,
      catalog: [changing],
    }).valid,
    false,
  );
  assert.equal(reads, 1);
});

test("stored array proxy errors become invalid layout results", () => {
  const input = new Proxy([item], {
    get(target, key) {
      if (key === "length") throw Error("private stored getter");
      return Reflect.get(target, key);
    },
  });
  assert.deepEqual(validateLayout(input, options), {
    valid: false,
    code: "LayoutShape",
  });
});

test("layout entry getters cannot extend the captured item limit during iteration", () => {
  const input = [
    {
      ...item,
      get width() {
        input.push({ ...item, id: "summary" });
        return 4;
      },
    },
  ];
  const result = validateLayout(input, { ...options, maxItems: 1 });
  assert.equal(result.valid, true);
  if (result.valid) assert.equal(result.layout.length, 1);
});
