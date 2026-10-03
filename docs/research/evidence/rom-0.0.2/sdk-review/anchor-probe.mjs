// Finite read-only query-envelope and float-token observations.
import assert from "node:assert/strict";
import { resolve, join } from "node:path";
import { pathToFileURL } from "node:url";
const root = resolve(process.argv[2]);
const load = (name) => import(pathToFileURL(join(root, "studio/src/lib/client", name)).href);
const { createClient } = await load("client.ts");
const { acceptedAnchor } = await load("query.ts");
const { parseWire, stringifyWire } = await load("codec.ts");
const observations = [];
const canonical = '{"version":1,"kind":"meter","schema_version":1,"filters":[],"comparisons":[],"order":[{"field":"value","direction":"asc"}],"id":"last","values":[{"state":"value","value":1.0}]}';
const query = { order: [{ field: "value", direction: "asc" }], limit: 5 };
const last = { key: { kind: "meter", id: "last" }, revision: 1n,
  value: parseWire('{"value":1.0,"hidden":"not needed"}'),
};
for (const [name, body] of [
  ["invalid nested tags", canonical.replace('"asc"', '"sideways"').replace('"state":"value"', '"state":"future"')],
  ["non-object filter", canonical.replace('"filters":[]', '"filters":[false]')],
  ["unknown envelope field", canonical.replace('"version":1', '"version":1,"unexpected":true')],
  ["missing anchor value", canonical.replace('"state":"value","value":1.0', '"state":"value"')],
]) {
  const boundary = acceptedAnchor(parseWire(body), "meter", "last");
  assert.equal(boundary.canonical, body);
  observations.push({ case: name, result: "accepted" });
}
{
  const bodies = [];
  const response = canonical.replace('"direction":"asc"', '"direction":"desc"');
  const client = createClient({ base: "/api", fetch: async (url, init) => {
    bodies.push(String(init.body));
    return new Response(String(url).endsWith("/query/anchor") ? response : "[]",
      { headers: { "content-type": "application/json" } });
  }});
  const anchor = await client.anchor(query, last);
  await client.query("meter", { ...query, after: structuredClone(anchor) });
  assert.match(bodies[1], /"after":.*"direction":"desc"/);
  observations.push({ case: "anchor response query binding", requested: "asc", acceptedAndResent: "desc" });
}
{
  const bodies = [];
  const client = createClient({ base: "/api", fetch: async (url, init) => {
    bodies.push(String(init.body));
    return new Response(String(url).endsWith("/query/anchor") ? canonical : "[]",
      { headers: { "content-type": "application/json" } });
  }});
  const anchor = await client.anchor(query, last);
  await client.query("meter", { ...query, after: structuredClone(anchor) });
  assert.match(bodies[0], /"value":1\.0/);
  assert.doesNotMatch(bodies[0], /hidden/);
  assert.match(bodies[1], /"value":1\.0/);
  observations.push({ case: "canonical clone positive", floatCategory: "preserved", unrelatedField: "removed" });
}
{
  const value = parseWire('{"number":1.0}');
  value.number = 2;
  assert.equal(stringifyWire(value), '{"number":2}');
  delete value.number;
  value.number = 1;
  assert.equal(stringifyWire(value), '{"number":1.0}');
  observations.push({ case: "deleted/recreated property", freshNumber: 1, serialized: stringifyWire(value) });
}
{
  const value = parseWire('[1.0,2]');
  value.reverse();
  assert.equal(stringifyWire(value), '[2,1]');
  observations.push({ case: "moved float array element", serialized: stringifyWire(value), floatCategory: "lost" });
}
{
  const value = parseWire('{"number":1.0}');
  value.number = 1n;
  assert.equal(stringifyWire(value), '{"number":1}');
  observations.push({ case: "explicit integer replacement", serialized: stringifyWire(value), oldFloatToken: "ignored" });
}
console.log(JSON.stringify({ node: process.version, observations }, null, 2));
