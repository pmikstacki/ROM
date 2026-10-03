// Read-only observations of the reviewed SDK. Pass the source checkout explicitly.
import assert from "node:assert/strict";
import { resolve, join } from "node:path";
import { pathToFileURL } from "node:url";
import { setTimeout as delay } from "node:timers/promises";

const root = resolve(process.argv[2]);
const load = (name) => import(pathToFileURL(join(root, "studio/src/lib/client", name)).href);
const { createClient } = await load("client.ts");
const { parseWire, stringifyWire } = await load("codec.ts");
const view = (revision = "1", id = "a") =>
  `{"key":{"kind":"task","id":"${id}"},"revision":${revision},"value":{"done":false}}`;
const json = (body) => new Response(body, { headers: { "content-type": "application/json" } });
const observations = [];

// Keep every pending fixture releasable. No network, credentials, or product writes.
{
  let finish;
  let signal;
  const client = createClient({ base: "/api", timeoutMs: 5,
    fetch: async (_, init) => {
      signal = init.signal;
      return new Promise((resolve) => { finish = resolve; });
    },
  });
  let status = "pending";
  const pending = client.read("task", "a").then(
    () => { status = "accepted"; }, () => { status = "rejected"; },
  );
  await delay(40);
  assert.equal(signal.aborted, true);
  assert.equal(status, "pending");
  finish(json(view()));
  await pending;
  assert.equal(status, "accepted");
  observations.push({ case: "POST deadline", after40ms: "pending", lateReply: status });
}
{
  const aborted = new AbortController();
  aborted.abort(Error("caller cancelled"));
  let calls = 0;
  const client = createClient({ base: "/api", fetch: async () => {
    calls++;
    return json(view());
  }});
  const result = await client.read("task", "a", aborted.signal);
  assert.equal(result.revision, 1n);
  assert.equal(calls, 1);
  observations.push({ case: "already cancelled POST", calls, reply: "accepted" });
}
{
  let release;
  const body = new ReadableStream({
    start(controller) { controller.enqueue(new Uint8Array(128)); },
    cancel() { return new Promise((resolve) => { release = resolve; }); },
  });
  const client = createClient({ base: "/api", maxBytes: 64, timeoutMs: 5,
    fetch: async () => new Response(body, { headers: { "content-type": "application/json" } }),
  });
  let status = "pending";
  const pending = client.read("task", "a").then(
    () => { status = "accepted"; }, () => { status = "rejected"; },
  );
  await delay(40);
  assert.equal(status, "pending");
  assert.equal(typeof release, "function");
  release();
  await pending;
  assert.equal(status, "rejected");
  observations.push({ case: "oversized body with pending cancel", after40ms: "pending", afterRelease: status });
}
{
  const client = createClient({ base: "/api", fetch: async () => json(view("1.0")) });
  assert.equal((await client.read("task", "a")).revision, 1n);
  observations.push({ case: "floating revision metadata", wire: "1.0", accepted: "1n" });
}
{
  const client = createClient({ base: "/api", fetch: async () => json(view("3")) });
  const mutation = client.prepare({ kind: "task", id: "a", expected: 7n,
    idempotency: "review-only", operation: { type: "delete" },
  });
  const result = await client.submit(mutation);
  assert.equal(result.revision, 3n);
  assert.equal(mutation.state, "succeeded");
  assert.notEqual(result.value, null);
  observations.push({ case: "delete correspondence", expected: "7n", result: "3n", value: "non-null", state: mutation.state });
}
{
  const client = createClient({ base: "/api", maxRows: 10,
    fetch: async () => json(`[${view("1", "a")},${view("1", "b")}]`),
  });
  assert.equal((await client.query("task", { limit: 1 })).length, 2);
  observations.push({ case: "query response count", requested: 1, accepted: 2 });
}
{
  const original = parseWire('{"value":1.0,"nested":[2e0]}');
  assert.equal(stringifyWire(original), '{"value":1.0,"nested":[2e0]}');
  assert.equal(stringifyWire(structuredClone(original)), '{"value":1,"nested":[2]}');
  assert.equal(stringifyWire(parseWire("1.0")), "1");
  observations.push({ case: "float token scope", original: stringifyWire(original), cloned: stringifyWire(structuredClone(original)), scalar: stringifyWire(parseWire("1.0")) });
}
{
  const body = view().replace('"done":false', '"text":"' + "a".repeat(1100000) + '"');
  const client = createClient({ base: "/api", maxBytes: 2 * 1024 * 1024,
    fetch: async () => json(body),
  });
  await assert.rejects(client.read("task", "a"), /wire byte limit/);
  observations.push({ case: "configured 2MiB response", actualBytes: new TextEncoder().encode(body).length, result: "wire byte limit at default 1MiB parser" });
}
{
  const client = createClient({ base: "/api", timeoutMs: Number.MAX_SAFE_INTEGER,
    fetch: async (_, init) => {
      await delay(10);
      assert.equal(init.signal.aborted, true);
      return json(view());
    },
  });
  await client.read("task", "a");
  observations.push({ case: "unrepresentable timer", configured: Number.MAX_SAFE_INTEGER, after10ms: "aborted (Node clamps to 1ms)" });
}
{
  const client = createClient({ base: "/api", fetch: async () => json('{"version":1.0,"resources":[]}') });
  assert.equal((await client.discover()).version, 1);
  observations.push({ case: "floating protocol version", wire: "1.0", accepted: 1 });
}
{
  const bad = createClient({ base: "/api", fetch: async () => new Response(
    new Uint8Array([0xc3, 0x28]), { headers: { "content-type": "application/json" } },
  )});
  await assert.rejects(bad.read("task", "a"), /encoded data/);
  assert.throws(() => parseWire('{"revision":1,"revision":2}'), /duplicate/);
  observations.push({ case: "negative controls", malformedUtf8: "rejected", duplicateMetadata: "rejected" });
}
console.log(JSON.stringify({ node: process.version, observations }, null, 2));
