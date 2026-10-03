import { test } from "node:test";
import assert from "node:assert/strict";
import { createClient } from "../../src/lib/client/client.ts";

const request = {
  kind: "task",
  id: "a",
  expected: null,
  idempotency: "key",
  operation: { type: "create", input: { title: "x" } },
} as const;
const view =
  '{"key":{"kind":"task","id":"a"},"revision":18446744073709551615,"value":{"title":"x"}}';
test("binds result identity and preserves revision, never accepts wrong resource", async () => {
  let response = view;
  const client = createClient({
    base: "/api",
    fetch: async () =>
      new Response(response, {
        headers: { "content-type": "application/json" },
      }),
  });
  assert.equal(
    (await client.read("task", "a")).revision,
    18446744073709551615n,
  );
  response = view.replace('"a"', '"other"');
  await assert.rejects(client.read("task", "a"), /identity/);
});
test("lost response retains exact frozen request and key for explicit retry", async () => {
  const bodies: string[] = [];
  const client = createClient({
    base: "/api",
    csrf: () => "csrf",
    fetch: async (_, init) => {
      assert.equal(new Headers(init?.headers).get("x-rom-csrf"), "csrf");
      bodies.push(String(init?.body));
      if (bodies.length === 1) throw new TypeError("connection lost");
      return new Response(view, {
        headers: { "content-type": "application/json" },
      });
    },
  });
  const mutable = structuredClone(request);
  const pending = client.prepare(mutable);
  mutable.operation.input.title = "changed";
  await assert.rejects(client.submit(pending));
  assert.equal(pending.state, "unknown");
  await client.submit(pending);
  assert.equal(pending.state, "succeeded");
  assert.equal(bodies[0], bodies[1]);
  assert.match(bodies[0], /"title":"x"/);
});
test("session generation rejects stale replies and old mutation retries", async () => {
  let resolve!: (value: Response) => void;
  const client = createClient({
    base: "/api",
    fetch: async () => new Promise((r) => (resolve = r)),
  });
  const pending = client.prepare(request);
  const reading = client.read("task", "a");
  client.invalidateSession();
  resolve(
    new Response(view, { headers: { "content-type": "application/json" } }),
  );
  await assert.rejects(reading, /session/);
  await assert.rejects(client.submit(pending), /session/);
});
test("denial is rejected, conflict is distinct and unknown outcome is recoverable", async () => {
  let status = 403,
    category = "denied";
  const client = createClient({
    base: "/api",
    fetch: async () =>
      new Response(JSON.stringify({ error: category }), {
        status,
        headers: { "content-type": "application/json" },
      }),
  });
  const denied = client.prepare(request);
  await assert.rejects(client.submit(denied));
  assert.equal(denied.state, "rejected");
  status = 409;
  category = "conflict";
  const conflict = client.prepare(request);
  await assert.rejects(client.submit(conflict));
  assert.equal(conflict.state, "conflict");
  status = 503;
  category = "outcome_unknown";
  const unknown = client.prepare(request);
  await assert.rejects(client.submit(unknown));
  assert.equal(unknown.state, "unknown");
});
test("response bounds, content type and rows are enforced", async () => {
  let response = () =>
    new Response("x".repeat(500), {
      headers: { "content-type": "application/json" },
    });
  const client = createClient({
    base: "/api",
    maxBytes: 200,
    maxRows: 1,
    fetch: async () => response(),
  });
  await assert.rejects(client.read("task", "a"), /limit/);
  response = () =>
    new Response(view, { headers: { "content-type": "text/html" } });
  await assert.rejects(client.read("task", "a"), /content/);
  response = () =>
    new Response(`[${view},${view}]`, {
      headers: { "content-type": "application/json" },
    });
  await assert.rejects(client.query("task", {}), /limit/);
});
test("public mutation status cannot bypass private ownership or reuse a failed attempt", async () => {
  let calls = 0;
  const c = createClient({
    base: "/api",
    fetch: async () => {
      calls++;
      return new Response(view, {
        headers: { "content-type": "application/json" },
      });
    },
  });
  const pending = c.prepare(request);
  c.invalidateSession();
  pending.generation = c.generation;
  await assert.rejects(c.submit(pending), /session/);
  assert.equal(calls, 0);
});
test("an unclassified server failure remains unknown and must not imply rollback", async () => {
  const c = createClient({
    base: "/api",
    fetch: async () =>
      new Response('{"error":"internal"}', {
        status: 500,
        headers: { "content-type": "application/json" },
      }),
  });
  const pending = c.prepare(request);
  await assert.rejects(c.submit(pending));
  assert.equal(pending.state, "unknown");
});

test("receipt replay rechecks current server authority rather than returning a cached result", async () => {
  let allowed = true;
  const client = createClient({
    base: "/api",
    fetch: async () =>
      allowed
        ? new Response(view, {
            headers: { "content-type": "application/json" },
          })
        : new Response('{"error":"denied"}', {
            status: 403,
            headers: { "content-type": "application/json" },
          }),
  });
  const pending = client.prepare(request);
  await client.submit(pending);
  allowed = false;
  await assert.rejects(client.submit(pending), /denied/);
  assert.equal(pending.state, "rejected");
  assert.equal(pending.result, undefined);
});
