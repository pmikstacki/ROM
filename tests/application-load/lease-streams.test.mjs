import test from "node:test";
import assert from "node:assert/strict";
import { openLeaseStreams } from "./lease-streams.mjs";
const session = { cookie: "rom_session=synthetic", csrf: "synthetic" };
const identity = {
  authenticated: true,
  user_id: "fixture-user",
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
function stream(frames, end = false) {
  return new Response(
    new ReadableStream({
      start(c) {
        for (const frame of frames) c.enqueue(new TextEncoder().encode(frame));
        if (end) c.close();
      },
    }),
    { headers: { "content-type": "text/event-stream" } },
  );
}
const tick = () => new Promise((r) => setImmediate(r));
test("exact expiry then EOF revalidates same session and requires actual fresh data", async () => {
  let sessions = 0,
    live = 0;
  const requests = [];
  const acquire = async (url, options) => {
    requests.push({ url, csrf: options.headers["x-rom-csrf"] });
    if (url.endsWith("/auth/session")) {
      sessions++;
      return Response.json(identity);
    }
    live++;
    return live === 1
      ? stream([data, "event: error\ndata: identity_expired\n\n"], true)
      : stream([data]);
  };
  const s = await openLeaseStreams(session, { normal: 1, slow: 0 }, acquire);
  for (let i = 0; i < 20 && live < 2; i++) await tick();
  const x = await s.close();
  assert.equal(x.error_events, 0);
  assert.equal(x.expected_expiries, 1);
  assert.equal(x.recoveries, 1);
  assert.equal(x.fresh_snapshots, 1);
  assert.equal(sessions, 2);
  assert.equal(live, 2);
  assert.equal(requests.at(-1).csrf, "fresh");
  assert.equal(x.additional_http.session_requests, 2);
  assert.equal(x.additional_http.live_requests, 2);
  assert.equal(x.readers_drained, true);
});
for (const [name, frames] of [
  ["arbitrary EOF", [data]],
  ["overload", [data, "event: error\ndata: overloaded\n\n"]],
  [
    "disclosure after expiry",
    [data, "event: error\ndata: identity_expired\n\n", data],
  ],
  [
    "repeated expiry",
    [
      data,
      "event: error\ndata: identity_expired\n\n",
      "event: error\ndata: identity_expired\n\n",
    ],
  ],
  ["malformed frame", [data, "event: data\ndata: {invalid}\n\n"]],
])
  test(`${name} never reacquires or hides failure`, async () => {
    let live = 0,
      sessions = 0;
    const s = await openLeaseStreams(
      session,
      { normal: 1, slow: 0 },
      async (url) => {
        if (url.endsWith("/auth/session")) {
          sessions++;
          return Response.json(identity);
        }
        live++;
        return stream(frames, true);
      },
    );
    await tick();
    const x = await s.close();
    assert.equal(x.recoveries, 0);
    assert.ok(x.error_events > 0);
    assert.equal(live, 1);
    assert.equal(sessions, 1);
  });
for (const [name, changed] of [
  ["revoked", { authenticated: false }],
  ["principal changed", { ...identity, user_id: "other" }],
  ["generation changed", { ...identity, generation: "different" }],
  [
    "original expiry extended",
    { ...identity, expires_at: identity.expires_at + 1 },
  ],
  ["unsafe expiry", { ...identity, expires_at: Number.MAX_SAFE_INTEGER + 1 }],
  ["missing CSRF", { ...identity, csrf_token: "" }],
])
  test(`recovery rejects ${name}`, async () => {
    let live = 0,
      sessions = 0;
    const s = await openLeaseStreams(
      session,
      { normal: 1, slow: 0 },
      async (url) => {
        if (url.endsWith("/auth/session"))
          return Response.json(++sessions === 1 ? identity : changed);
        live++;
        return stream([data, "event: error\ndata: identity_expired\n\n"], true);
      },
    );
    await tick();
    const x = await s.close();
    assert.equal(live, 1);
    assert.equal(sessions, 2);
    assert.equal(x.recoveries, 0);
    assert.equal(x.abandoned_recoveries, 1);
    assert.ok(x.error_events > 0);
    assert.ok(!JSON.stringify(x).includes("fixture-user"));
  });
test("repeated valid cycles exhaust finite recovery budget without another acquisition", async () => {
  let live = 0,
    sessions = 0;
  const s = await openLeaseStreams(
    session,
    { normal: 1, slow: 0 },
    async (url) => {
      if (url.endsWith("/auth/session")) {
        sessions++;
        return Response.json(identity);
      }
      live++;
      return stream([data, "event: error\ndata: identity_expired\n\n"], true);
    },
  );
  for (let i = 0; i < 20; i++) await tick();
  const x = await s.close();
  assert.equal(live, 6);
  assert.equal(sessions, 6);
  assert.equal(x.recoveries, 5);
  assert.equal(x.error_events, 1);
  assert.equal(x.error_classes["recovery-budget"], 1);
});
test("cancel while session revalidation waits aborts it and records abandoned recovery", async () => {
  let calls = 0,
    waiting = false;
  const s = await openLeaseStreams(
    session,
    { normal: 1, slow: 0 },
    async (url, o) => {
      if (url.endsWith("/auth/session")) {
        if (++calls === 1) return Response.json(identity);
        waiting = true;
        return new Promise((_, reject) =>
          o.signal.addEventListener("abort", () => reject(o.signal.reason), {
            once: true,
          }),
        );
      }
      return stream([data, "event: error\ndata: identity_expired\n\n"], true);
    },
  );
  for (let i = 0; i < 20 && !waiting; i++) await tick();
  const x = await s.close();
  assert.equal(waiting, true);
  assert.equal(x.cancelled_recoveries, 1);
  assert.equal(x.error_events, 0);
  assert.equal(x.recoveries, 0);
  assert.equal(x.readers_drained, true);
});
test("fresh stream without expected row snapshot cannot satisfy recovery", async () => {
  let live = 0;
  const s = await openLeaseStreams(
    session,
    { normal: 1, slow: 0 },
    async (url) =>
      url.endsWith("/auth/session")
        ? Response.json(identity)
        : ++live === 1
          ? stream([data, "event: error\ndata: identity_expired\n\n"], true)
          : stream(["event: data\ndata: []\n\n"], true),
  );
  await tick();
  const x = await s.close();
  assert.equal(x.recoveries, 0);
  assert.equal(x.abandoned_recoveries, 1);
  assert.ok(x.error_events > 0);
});
test("slow response remains unread and is cancelled with all normal reader ownership", async () => {
  let normal = 0,
    cancels = 0;
  const s = await openLeaseStreams(
    session,
    { normal: 4, slow: 1 },
    async (url) => {
      if (url.endsWith("/auth/session")) return Response.json(identity);
      const index = normal++;
      return new Response(
        new ReadableStream({
          start(c) {
            if (index < 4)
              c.enqueue(
                new TextEncoder().encode(
                  data
                    .replace(
                      "load-00000",
                      `load-${String(index).padStart(5, "0")}`,
                    )
                    .replace(
                      "Synthetic load row 00000",
                      `Synthetic load row ${String(index).padStart(5, "0")}`,
                    ),
                ),
              );
          },
          cancel() {
            cancels++;
          },
        }),
        { headers: { "content-type": "text/event-stream" } },
      );
    },
  );
  await tick();
  const x = await s.close();
  assert.equal(x.data_events, 4);
  assert.equal(x.error_events, 0);
  assert.equal(x.readers_drained, true);
  assert.equal(cancels, 5);
  assert.equal(x.slow_reads, 0);
});
test(
  "successful fresh snapshot clears recovery deadline while the new lease stays readable",
  { timeout: 12000 },
  async () => {
    let live = 0;
    const s = await openLeaseStreams(
      session,
      { normal: 1, slow: 0 },
      async (url) =>
        url.endsWith("/auth/session")
          ? Response.json(identity)
          : ++live === 1
            ? stream([data, "event: error\ndata: identity_expired\n\n"], true)
            : stream([data]),
    );
    try {
      await tick();
      assert.equal(s.counts().recoveries, 1);
      await new Promise((resolve) => setTimeout(resolve, 10500));
      assert.equal(s.counts().error_events, 0);
    } finally {
      assert.equal((await s.close()).readers_drained, true);
    }
  },
);
test("nonfinite wall clock cannot admit a session baseline", async () => {
  const original = Date.now;
  Date.now = () => NaN;
  let calls = 0;
  try {
    await assert.rejects(
      openLeaseStreams(session, { normal: 1, slow: 0 }, async () => {
        calls++;
        return Response.json(identity);
      }),
      /lease observation refused/,
    );
    assert.equal(calls, 1);
  } finally {
    Date.now = original;
  }
});
test("late session response after cancellation cannot dispatch fresh live acquisition", async () => {
  let calls = 0,
    live = 0,
    release;
  const s = await openLeaseStreams(
    session,
    { normal: 1, slow: 0 },
    async (url) => {
      if (url.endsWith("/auth/session"))
        return ++calls === 1
          ? Response.json(identity)
          : new Promise((resolve) => {
              release = () => resolve(Response.json(identity));
            });
      live++;
      return stream([data, "event: error\ndata: identity_expired\n\n"], true);
    },
  );
  await tick();
  const closing = s.close();
  release();
  const x = await closing;
  assert.equal(live, 1);
  assert.equal(x.recoveries, 0);
  assert.equal(x.cancelled_recoveries, 1);
  assert.equal(x.readers_drained, true);
});
test("incomplete UTF8 after expiry is not a valid terminal EOF", async () => {
  let live = 0;
  const s = await openLeaseStreams(
    session,
    { normal: 1, slow: 0 },
    async (url) =>
      url.endsWith("/auth/session")
        ? Response.json(identity)
        : (++live,
          new Response(
            new ReadableStream({
              start(c) {
                c.enqueue(
                  new TextEncoder().encode(
                    data + "event: error\ndata: identity_expired\n\n",
                  ),
                );
                c.enqueue(new Uint8Array([0xe2]));
                c.close();
              },
            }),
            { headers: { "content-type": "text/event-stream" } },
          )),
  );
  await tick();
  const x = await s.close();
  assert.equal(live, 1);
  assert.equal(x.terminal_eofs, 0);
  assert.ok(x.error_events > 0);
});
test(
  "fresh data timeout is abandoned recovery and cannot count as successful reacquisition",
  { timeout: 12000 },
  async () => {
    let live = 0;
    const s = await openLeaseStreams(
      session,
      { normal: 1, slow: 0 },
      async (url) =>
        url.endsWith("/auth/session")
          ? Response.json(identity)
          : ++live === 1
            ? stream([data, "event: error\ndata: identity_expired\n\n"], true)
            : stream([]),
    );
    try {
      await new Promise((resolve) => setTimeout(resolve, 10500));
      const x = s.counts();
      assert.equal(x.recoveries, 0);
      assert.equal(x.abandoned_recoveries, 1);
      assert.equal(x.error_events, 1);
    } finally {
      assert.equal((await s.close()).readers_drained, true);
    }
  },
);
test("aggregate SSE byte cap rejects an oversized body before recovery", async () => {
  const s = await openLeaseStreams(
    session,
    { normal: 1, slow: 0 },
    async (url) =>
      url.endsWith("/auth/session")
        ? Response.json(identity)
        : new Response(
            new ReadableStream({
              start(c) {
                c.enqueue(new TextEncoder().encode(data));
                c.enqueue(new Uint8Array(4194305));
                c.close();
              },
            }),
            { headers: { "content-type": "text/event-stream" } },
          ),
  );
  await tick();
  const x = await s.close();
  assert.equal(x.error_classes["byte-budget"], 1);
  assert.equal(x.recoveries, 0);
});
test("transport errors cannot inject diagnostic category secrets", async () => {
  let calls = 0;
  const s = await openLeaseStreams(
    session,
    { normal: 1, slow: 0 },
    async (url) => {
      if (url.endsWith("/auth/session")) {
        if (++calls === 1) return Response.json(identity);
        throw Object.assign(new Error("SECRET-body"), {
          leaseCode: "SECRET-category",
        });
      }
      return stream([data, "event: error\ndata: identity_expired\n\n"], true);
    },
  );
  await tick();
  const x = await s.close();
  assert.equal(x.error_classes.transport, 1);
  assert.ok(!JSON.stringify(x).includes("SECRET"));
});
