import test from "node:test";
import assert from "node:assert/strict";
import { openLeaseStreams } from "./lease-streams.mjs";
const session = { cookie: "rom_session=synthetic", csrf: "synthetic" };
const identity = {
  authenticated: true,
  user_id: "owner",
  generation: "original",
  expires_at: Math.floor(Date.now() / 1000) + 300,
  csrf_token: "fresh",
};
const data =
  "event: data\ndata: " +
  JSON.stringify([
    {
      key: { kind: "load-records", id: "load-00000" },
      revision: 1,
      value: { title: "Synthetic load row 00000", counter: 0, open: true },
    },
  ]) +
  "\n\n";
test("late cancelled slow acquisition retains and cancels its response body", async () => {
  const abort = new AbortController();
  let live = 0,
    cancelled = 0;
  await assert.rejects(
    openLeaseStreams(
      session,
      { normal: 1, slow: 1 },
      async (url) => {
        if (url.endsWith("/auth/session")) return Response.json(identity);
        const index = live++;
        const response = new Response(
          new ReadableStream({
            start(c) {
              if (index === 0) c.enqueue(new TextEncoder().encode(data));
            },
            cancel() {
              cancelled++;
            },
          }),
          { headers: { "content-type": "text/event-stream" } },
        );
        if (index === 1) {
          abort.abort();
          await new Promise((r) => setImmediate(r));
        }
        return response;
      },
      { signal: abort.signal },
    ),
  );
  assert.equal(cancelled, 2);
});
test("conflicting duplicate event fields cannot authorize recovery", async () => {
  let live = 0;
  const s = await openLeaseStreams(
    session,
    { normal: 1, slow: 0 },
    async (url) => {
      if (url.endsWith("/auth/session")) return Response.json(identity);
      live++;
      return new Response(
        new ReadableStream({
          start(c) {
            c.enqueue(new TextEncoder().encode(data));
            if (live === 1) {
              c.enqueue(
                new TextEncoder().encode(
                  "event: error\nevent: data\ndata: identity_expired\n\n",
                ),
              );
              c.close();
            }
          },
        }),
        { headers: { "content-type": "text/event-stream" } },
      );
    },
  );
  try {
    await new Promise((r) => setImmediate(r));
    assert.equal(live, 1);
    assert.equal(s.counts().recoveries, 0);
    assert.ok(s.counts().error_events > 0);
  } finally {
    await s.close();
  }
});
