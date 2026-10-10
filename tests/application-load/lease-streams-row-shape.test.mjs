import test from "node:test";
import assert from "node:assert/strict";
import { openLeaseStreams } from "./lease-streams.mjs";
const identity = {
  authenticated: true,
  user_id: "fixture-user",
  generation: "original",
  expires_at: Math.floor(Date.now() / 1000) + 300,
  csrf_token: "fresh",
};
const row = () => ({
  title: "Synthetic load row 00000",
  counter: 0,
  open: true,
});
const frame = (value) =>
  "event: data\ndata: " +
  JSON.stringify([
    { key: { kind: "load-records", id: "load-00000" }, revision: 1, value },
  ]) +
  "\n\n";
function stream(value, end) {
  return new Response(
    new ReadableStream({
      start(c) {
        c.enqueue(new TextEncoder().encode(frame(value)));
        if (end) {
          c.enqueue(
            new TextEncoder().encode(
              "event: error\ndata: identity_expired\n\n",
            ),
          );
          c.close();
        }
      },
    }),
    { headers: { "content-type": "text/event-stream" } },
  );
}
for (const [name, change] of [
  ["empty", () => ({})],
  ["array", () => []],
  [
    "missing title",
    (r) => {
      delete r.title;
      return r;
    },
  ],
  ["numeric title", (r) => ({ ...r, title: 0 })],
  [
    "wrong selected title",
    (r) => ({ ...r, title: "Synthetic load row 00001" }),
  ],
  [
    "missing counter",
    (r) => {
      delete r.counter;
      return r;
    },
  ],
  ["unsafe counter", (r) => ({ ...r, counter: Number.MAX_SAFE_INTEGER + 1 })],
  ["fractional counter", (r) => ({ ...r, counter: 0.5 })],
  ["negative counter", (r) => ({ ...r, counter: -1 })],
  ["over profile counter", (r) => ({ ...r, counter: 12001 })],
  [
    "missing open",
    (r) => {
      delete r.open;
      return r;
    },
  ],
  ["nonboolean open", (r) => ({ ...r, open: 1 })],
])
  test("recovered snapshot rejects " + name + " domain fields", async () => {
    let live = 0;
    const s = await openLeaseStreams(
      { cookie: "rom_session=synthetic", csrf: "synthetic" },
      { normal: 1, slow: 0 },
      async (url) =>
        url.endsWith("/auth/session")
          ? Response.json(identity)
          : stream(++live === 1 ? row() : change(row()), live === 1),
    );
    try {
      for (let i = 0; i < 20; i++) await new Promise((r) => setImmediate(r));
      const counts = s.counts();
      assert.equal(live, 2);
      assert.equal(counts.recoveries, 0);
      assert.equal(counts.fresh_snapshots, 0);
      assert.ok(counts.error_events > 0);
    } finally {
      await s.close();
    }
  });
test("full authorized row retains bounded counter and boolean at profile maximum", async () => {
  let live = 0;
  const s = await openLeaseStreams(
    { cookie: "rom_session=synthetic", csrf: "synthetic" },
    { normal: 1, slow: 0 },
    async (url) =>
      url.endsWith("/auth/session")
        ? Response.json(identity)
        : stream(
            ++live === 1 ? row() : { ...row(), counter: 12000, open: false },
            live === 1,
          ),
  );
  try {
    for (let i = 0; i < 20; i++) await new Promise((r) => setImmediate(r));
    assert.equal(s.counts().recoveries, 1);
    assert.equal(s.counts().error_events, 0);
  } finally {
    await s.close();
  }
});
