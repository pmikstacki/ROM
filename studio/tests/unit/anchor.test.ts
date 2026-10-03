import { test } from "node:test";
import assert from "node:assert/strict";
import { createClient } from "../../src/lib/client/client.ts";
import { parseWire, stringifyWire } from "../../src/lib/client/codec.ts";
import { acceptedAnchor, queryWire } from "../../src/lib/client/query.ts";
import type { QuerySpec } from "../../src/lib/client/types.ts";
const last = {
  key: { kind: "meter", id: "last" },
  revision: 1n,
  value: parseWire('{"value":1.0,"large":"unused"}') as never,
};
const query = {
  order: [{ field: "value", direction: "asc" as const }],
  limit: 5,
};
const canonical =
  '{"version":1,"kind":"meter","schema_version":1,"filters":[],"comparisons":[],"order":[{"field":"value","direction":"asc"}],"id":"last","values":[{"state":"value","value":1.0}]}';
test("canonical query boundary survives cloning and float integer replay", async () => {
  const bodies: string[] = [];
  const c = createClient({
    base: "/api",
    fetch: async (url, init) => {
      bodies.push(String(init?.body));
      return new Response(
        String(url).endsWith("/query/anchor") ? canonical : "[]",
        { headers: { "content-type": "application/json" } },
      );
    },
  });
  const boundary = await c.anchor(query, last);
  assert.equal(boundary.kind, "meter");
  await c.query("meter", { ...query, after: structuredClone(boundary) });
  assert.match(bodies[1], /"value":1\.0/);
  assert.doesNotMatch(bodies[1], /canonical/);
  assert.doesNotMatch(bodies[0], /unused/);
});
test("anchors reject journal cursors and response identity changes", async () => {
  let value = canonical;
  const c = createClient({
    base: "/api",
    fetch: async () =>
      new Response(value, { headers: { "content-type": "application/json" } }),
  });
  value = canonical.replace('"last"', '"wrong"');
  await assert.rejects(c.anchor(query, last), /identity/);
  await assert.rejects(
    c.query("meter", {
      ...query,
      after: { kind: "meter", generation: "x", position: 1 } as never,
    }),
    /anchor/,
  );
});

test("anchor admission rejects malformed native envelope members", () => {
  for (const malformed of [
    canonical.replace('"version":1', '"version":1,"unexpected":true'),
    canonical.replace('"direction":"asc"', '"direction":"sideways"'),
    canonical.replace('"field":"value"', '"field":null'),
    canonical.replace('"direction":"asc"', '"direction":"asc","extra":true'),
    canonical.replace('"filters":[]', '"filters":[false]'),
    canonical.replace('"filters":[]', '"filters":[{"field":"value"}]'),
    canonical.replace(
      '"filters":[]',
      '"filters":[{"field":"value","value":null,"absent":"yes"}]',
    ),
    canonical.replace(
      '"comparisons":[]',
      '"comparisons":[{"field":"value","value":1,"op":"contains"}]',
    ),
    canonical.replace('"state":"value"', '"state":"future"'),
    canonical.replace('"state":"value","value":1.0', '"state":"value"'),
    canonical.replace(
      '"state":"value","value":1.0',
      '"state":"missing","value":null',
    ),
    canonical.replace(
      '"state":"value","value":1.0',
      '"state":"missing","extra":true',
    ),
  ]) {
    assert.throws(
      () => acceptedAnchor(parseWire(malformed), "meter", "last"),
      /anchor/,
      malformed,
    );
  }
});

test("anchor replies bind the requested literal order and predicate structure", () => {
  const response = parseWire(
    canonical.replace('"direction":"asc"', '"direction":"desc"'),
  );
  assert.throws(
    () => acceptedAnchor(response, "meter", "last", query),
    /anchor/,
  );
  const withPredicate = parseWire(
    canonical.replace(
      '"filters":[]',
      '"filters":[{"field":"value","value":1.0,"absent":false}]',
    ),
  );
  assert.throws(
    () => acceptedAnchor(withPredicate, "meter", "last", query),
    /anchor/,
  );
  assert.throws(
    () =>
      queryWire("meter", {
        ...query,
        after: {
          kind: "meter",
          id: "last",
          schema_version: 1,
          canonical: canonical.replace(
            '"direction":"asc"',
            '"direction":"desc"',
          ),
        },
      }),
    /anchor/,
  );
});

test("anchor correspondence leaves canonical codec operands opaque", () => {
  const requested: QuerySpec = {
    ...query,
    filters: [{ field: "value", value: "INPUT" }],
    comparisons: [{ field: "value", op: "ge", value: "INPUT", absent: false }],
  };
  const response = canonical
    .replace(
      '"filters":[]',
      '"filters":[{"field":"value","value":{"custom":[1.0]},"absent":false}]',
    )
    .replace(
      '"comparisons":[]',
      '"comparisons":[{"field":"value","op":"ge","value":"normalized"}]',
    );
  const accepted = acceptedAnchor(
    parseWire(response),
    "meter",
    "last",
    requested,
  );
  assert.equal(accepted.canonical, response);
  assert.match(
    stringifyWire(
      queryWire("meter", { ...requested, after: structuredClone(accepted) }),
    ),
    /"custom":\[1\.0\]/,
  );
  for (const changed of [
    response.replace('"op":"ge"', '"op":"gt"'),
    response.replace('"absent":false', '"absent":true'),
    response.replace('"field":"value","op"', '"field":"other","op"'),
  ])
    assert.throws(
      () => acceptedAnchor(parseWire(changed), "meter", "last", requested),
      /anchor/,
    );
});

test("anchor value states keep missing distinct from present null", () => {
  const missing = canonical.replace(
    '"state":"value","value":1.0',
    '"state":"missing"',
  );
  const presentNull = canonical.replace('"value":1.0', '"value":null');
  assert.equal(
    acceptedAnchor(parseWire(missing), "meter", "last", query).canonical,
    missing,
  );
  assert.equal(
    acceptedAnchor(parseWire(presentNull), "meter", "last", query).canonical,
    presentNull,
  );
});

test("native comparison operators cannot be coerced from arrays", () => {
  const malformed = canonical.replace(
    '"comparisons":[]',
    '"comparisons":[{"field":"value","value":1,"op":["eq"]}]',
  );
  assert.throws(
    () => acceptedAnchor(parseWire(malformed), "meter", "last"),
    /anchor/,
  );
});

test("native absence predicates retain their null and equality constraints", () => {
  for (const predicate of [
    '"filters":[{"field":"value","value":1,"absent":true}]',
    '"comparisons":[{"field":"value","value":null,"absent":true,"op":"ge"}]',
  ]) {
    const malformed = canonical.replace(
      predicate.startsWith('"filters"') ? '"filters":[]' : '"comparisons":[]',
      predicate,
    );
    assert.throws(
      () => acceptedAnchor(parseWire(malformed), "meter", "last"),
      /anchor/,
    );
  }
});

test("public anchor response is bound to the submitted snapshot despite caller edits", async () => {
  let resolve!: (value: Response) => void;
  const c = createClient({
    base: "/api",
    fetch: async () =>
      new Promise((r) => {
        resolve = r;
      }),
  });
  const mutable: QuerySpec = structuredClone(query);
  const pending = c.anchor(mutable, last);
  mutable.order![0].direction = "desc";
  resolve(
    new Response(canonical, {
      headers: { "content-type": "application/json" },
    }),
  );
  assert.equal((await pending).canonical, canonical);
  const mismatched = c.anchor(query, last);
  resolve(
    new Response(canonical.replace('"direction":"asc"', '"direction":"desc"'), {
      headers: { "content-type": "application/json" },
    }),
  );
  await assert.rejects(mismatched, /anchor/);
});
