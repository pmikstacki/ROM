import test from "node:test";
import assert from "node:assert/strict";
import {
  captureHistory,
  formatHistoryInstant,
} from "../../src/lib/ui/history.ts";
const entry = {
  id: '["local","human","alice"]',
  title: "Inspection",
  instant: Date.UTC(2026, 9, 8),
  count: 1000,
};

test("history preserves exact IDs and detaches bounded display data", () => {
  const input = [{ ...entry }];
  const result = captureHistory(input, 10);
  input[0].title = "changed";
  assert.equal(result[0].title, "Inspection");
  assert.equal(result[0].id, entry.id);
  assert.equal(Object.isFrozen(result), true);
  assert.equal(Object.isFrozen(result[0]), true);
});

test("history rejects duplicate IDs, excessive values and invalid dates", () => {
  for (const entries of [
    [entry, entry],
    [{ ...entry, id: "" }],
    [{ ...entry, title: "x".repeat(4097) }],
    [{ ...entry, instant: NaN }],
    [{ ...entry, count: -1 }],
  ]) {
    assert.throws(() => captureHistory(entries, 10), TypeError);
  }
  assert.throws(() => captureHistory([entry], 0), TypeError);
  assert.throws(
    () => captureHistory([entry, { ...entry, id: "second" }], 1),
    TypeError,
  );
});

test("history dates use explicit locale and timezone without changing stored instants", () => {
  const instant = entry.instant;
  assert.notEqual(
    formatHistoryInstant(instant, "en-US", "UTC"),
    formatHistoryInstant(instant, "pl-PL", "UTC"),
  );
  assert.notEqual(
    formatHistoryInstant(instant, "en-US", "UTC"),
    formatHistoryInstant(instant, "en-US", "America/New_York"),
  );
  assert.equal(entry.instant, instant);
});

test("history captures each value once before validating its admitted bounds", () => {
  let reads = 0;
  const input = {
    ...entry,
    get title() {
      return ++reads <= 3 ? "safe" : "x".repeat(5000);
    },
  };
  const result = captureHistory([input], 10);
  assert.equal(result[0].title, "safe");
  assert.equal(reads, 1);
});

test("history iterates only its captured entry count without calling an overridden map", () => {
  let reads = 0;
  const input = new Proxy(
    Array.from({ length: 201 }, (_, i) => ({ ...entry, id: String(i) })),
    {
      get(target, key) {
        return key === "length"
          ? ++reads === 1
            ? 1
            : 201
          : Reflect.get(target, key);
      },
    },
  );
  assert.equal(captureHistory(input, 1).length, 1);
  assert.equal(reads, 1);
  const overridden = [{ ...entry }];
  overridden.map = () => {
    throw Error("private caller method");
  };
  assert.equal(captureHistory(overridden, 1).length, 1);
});
