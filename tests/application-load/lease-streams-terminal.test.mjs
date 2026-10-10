import test from "node:test";
import assert from "node:assert/strict";
import { openLeaseStreams } from "./lease-streams.mjs";

for (const category of ["overloaded", "denied", "closed", "internal", "timeout", "SECRET_token"])
  test(`lease observation preserves bounded terminal attribution for ${category}`, async () => {
    let controller;
    const snapshot = [{
      key: { kind: "load-records", id: "load-00000" }, revision: 1,
      value: { title: "Synthetic load row 00000", counter: 0, open: true },
    }];
    const streams = await openLeaseStreams(
      { cookie: "rom_session=synthetic", csrf: "synthetic" },
      { normal: 1, slow: 0 },
      async url => url.endsWith("/auth/session")
        ? Response.json({ authenticated: true, user_id: "fixture", generation: "original",
          expires_at: Math.floor(Date.now() / 1000) + 300, csrf_token: "synthetic" })
        : new Response(new ReadableStream({ start(value) {
          controller = value;
          value.enqueue(new TextEncoder().encode(`event: data\ndata: ${JSON.stringify(snapshot)}\n\n`));
        }}), { headers: { "content-type": "text/event-stream" } }),
    );
    try {
      controller.enqueue(new TextEncoder().encode(`event: error\ndata: ${category}\n\n`));
      controller.close();
      for (let index = 0; index < 5; index++) await new Promise(resolve => setImmediate(resolve));
      const counts = streams.counts();
      assert.equal(counts.error_events, 1);
      assert.deepEqual(counts.error_classes, { [`server-terminal-${category === "SECRET_token" ? "other" : category}`]: 1 });
      assert.equal(counts.expected_expiries, 0);
      assert.equal(counts.reacquisition_attempts, 0);
      assert.equal(counts.recoveries, 0);
      assert.equal(counts.initial_snapshots, 1);
      assert.ok(!JSON.stringify(counts).includes("SECRET_token"));
    } finally {
      assert.equal((await streams.close()).readers_drained, true);
    }
  });
