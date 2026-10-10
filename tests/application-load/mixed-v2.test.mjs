import test from "node:test";
import assert from "node:assert/strict";
import {
  runMixedLoad,
  runMixedLeaseLoad,
  runSelectedMixedLoad,
  attachMixedHostProof,
} from "./mixed-run.mjs";
const session = { cookie: "rom_session=private-cookie", csrf: "private-csrf" };
function inputs() {
  const now = Date.now();
  return {
    session,
    operatorSession: session,
    seed: {
      resources: 10000,
      candidate_bytes: 10000,
      seed_work: 10000,
      done_work: 10000,
      work_bytes: 10000,
      drain_ms: 10,
      elapsed_ms: 10,
    },
    tokenTiming: {
      iat: Math.floor(now / 1000),
      exp: Math.floor(now / 1000) + 300,
      received_at_ms: now,
    },
    providerProfile: "exploratory",
  };
}
function* plans() {
  yield {
    index: 0,
    planned: 0,
    measured: true,
    requests: [
      {
        method: "POST",
        path: "/rom-studio/api/read",
        measurement: "read",
        body: { kind: "load-records", id: "load-00000" },
      },
    ],
  };
}
function transport() {
  let sessionCalls = 0,
    streamCalls = 0;
  const identity = {
    authenticated: true,
    user_id: "private-user",
    generation: "private-generation",
    expires_at: Math.floor(Date.now() / 1000) + 300,
    csrf_token: "fresh-csrf",
  };
  return {
    get sessionCalls() {
      return sessionCalls;
    },
    get streamCalls() {
      return streamCalls;
    },
    async acquire(url, options) {
      if (url.endsWith("/auth/session")) {
        sessionCalls++;
        return Response.json(identity);
      }
      if (url.endsWith("/live")) {
        streamCalls++;
        const title = JSON.parse(options.body).query.filters[0].value;
        const id = "load-" + title.slice(-5);
        return new Response(
          new ReadableStream({
            start(c) {
              c.enqueue(
                new TextEncoder().encode(
                  "event: data\ndata: " +
                    JSON.stringify([
                      {
                        key: { kind: "load-records", id },
                        revision: 1,
                        value: {
                          title: `Synthetic load row ${id.slice(-5)}`,
                          counter: 0,
                          open: true,
                        },
                      },
                    ]) +
                    "\n\n",
                ),
              );
            },
          }),
          { headers: { "content-type": "text/event-stream" } },
        );
      }
      if (url.endsWith("/capabilities"))
        return Response.json({
          protocol_version: 1,
          inspect: true,
          retry: true,
          reconcile: true,
        });
      if (url.endsWith("/list"))
        return Response.json({
          protocol_version: 1,
          records: [],
          cursor: null,
        });
      return Response.json({
        key: JSON.parse(options.body),
        revision: 1,
        value: {},
      });
    },
  };
}
test("v2 shares real workload runner while additional auth/stream traffic remains separate", async () => {
  const io = transport(),
    result = await runMixedLeaseLoad(inputs(), {
      acquire: io.acquire,
      plans: plans(),
    });
  assert.equal(result.schema, "rom-application-mixed-driver-v2");
  assert.equal(result.status, "completed");
  assert.equal(result.http.requests, 1);
  assert.equal(result.operator_http.requests, 4);
  assert.equal(result.streams.additional_http.session_requests, 1);
  assert.equal(result.streams.additional_http.live_requests, 5);
  assert.equal(io.sessionCalls, 1);
  assert.equal(io.streamCalls, 5);
  assert.equal(result.streams.error_events, 0);
  assert.equal(result.streams.readers_drained, true);
  for (const secret of [
    "private-user",
    "private-generation",
    "private-cookie",
    "fresh-csrf",
  ])
    assert.ok(!JSON.stringify(result).includes(secret));
});
test("v1 retains its schema and never adds auth/session recovery requests", async () => {
  const io = transport(),
    result = await runMixedLoad(inputs(), {
      acquire: io.acquire,
      plans: plans(),
    });
  assert.equal(result.schema, "rom-application-mixed-driver-v1");
  assert.equal(io.sessionCalls, 0);
  assert.equal(result.http.requests, 1);
  assert.equal(result.streams.additional_http, undefined);
});
test("closed version dispatch rejects unknown selection before requests", async () => {
  let calls = 0;
  await assert.rejects(
    runSelectedMixedLoad("unexpected", inputs(), {
      acquire: async () => {
        calls++;
      },
      plans: plans(),
    }),
    /version/,
  );
  assert.equal(calls, 0);
});
test("v2 cannot replace actual durable Work completion with fresh snapshots", () => {
  assert.throws(
    () =>
      attachMixedHostProof(
        { schema: "rom-application-mixed-driver-v2" },
        { records: 12000, bytes: 100, done: 11999 },
        {
          semantics: "fixture-commit-confirmation-to-claim-confirmation",
          restart_durable: false,
          nanosecond_samples: [],
          missing: 0,
          rejected: 0,
        },
        { rss_bytes: 1, disk_bytes: 1 },
      ),
    /complete/,
  );
});
