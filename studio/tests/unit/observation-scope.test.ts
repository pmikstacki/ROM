import test from "node:test";
import assert from "node:assert/strict";
import {
  captureObservationScope,
  sameObservationScope,
} from "../../src/lib/observe/scope.ts";

test("public observation requires an explicit stable public scope", () => {
  assert.deepEqual(
    captureObservationScope({ kind: "public", key: "public-catalog" }),
    { kind: "public", key: "public-catalog" },
  );
  for (const value of [
    null,
    undefined,
    {},
    { kind: "public", key: "" },
    { kind: "principal", principal: null, generation: 0 },
  ]) {
    assert.throws(() => captureObservationScope(value), /scope/i);
  }
});

test("principal scope preserves exact identity and a detached generation", () => {
  const principal = {
    authority: "local",
    kind: "human",
    subject: '["local","human","alice"]',
  };
  const input = { kind: "principal", principal, generation: 7 };
  const scope = captureObservationScope(input);
  principal.subject = "bob";
  input.generation = 8;
  assert.deepEqual(scope, {
    kind: "principal",
    principal: {
      authority: "local",
      kind: "human",
      subject: '["local","human","alice"]',
    },
    generation: 7,
  });
  assert.equal(Object.isFrozen(scope), true);
  assert.equal(
    Object.isFrozen(scope.kind === "principal" && scope.principal),
    true,
  );
});

test("authority renewal and delimiter-shaped identities are distinct scopes", () => {
  const one = captureObservationScope({
    kind: "principal",
    principal: { authority: "a", kind: "human", subject: "b/c" },
    generation: 1,
  });
  const two = captureObservationScope({
    kind: "principal",
    principal: { authority: "a/b", kind: "human", subject: "c" },
    generation: 1,
  });
  const renewed = captureObservationScope({
    kind: "principal",
    principal: { authority: "a", kind: "human", subject: "b/c" },
    generation: 2,
  });
  assert.equal(sameObservationScope(one, two), false);
  assert.equal(sameObservationScope(one, renewed), false);
  assert.equal(sameObservationScope(one, captureObservationScope(one)), true);
  assert.equal(
    sameObservationScope(
      one,
      captureObservationScope({ kind: "public", key: "a/b/c" }),
    ),
    false,
  );
});

test("malformed principal, generation and unbounded scope keys are rejected", () => {
  const valid = {
    kind: "principal",
    principal: { authority: "local", kind: "human", subject: "alice" },
    generation: 1,
  };
  for (const generation of [
    -1,
    NaN,
    Infinity,
    0.5,
    Number.MAX_SAFE_INTEGER + 1,
  ]) {
    assert.throws(
      () => captureObservationScope({ ...valid, generation }),
      /scope/i,
    );
  }
  assert.throws(
    () =>
      captureObservationScope({
        ...valid,
        principal: { ...valid.principal, kind: "anonymous" },
      }),
    /scope/i,
  );
  assert.throws(
    () => captureObservationScope({ kind: "public", key: "ą".repeat(513) }),
    /scope/i,
  );
});

test("inherited fields cannot silently establish an observation scope", () => {
  assert.throws(
    () =>
      captureObservationScope(Object.create({ kind: "public", key: "hidden" })),
    /scope/i,
  );
  assert.throws(
    () =>
      captureObservationScope({
        kind: "principal",
        generation: 0,
        principal: Object.create({
          authority: "local",
          kind: "human",
          subject: "alice",
        }),
      }),
    /scope/i,
  );
});
