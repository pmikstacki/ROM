import test from "node:test";
import assert from "node:assert/strict";
import { runMixedLoad, attachMixedHostProof } from "./mixed-run.mjs";
const session = { cookie: "rom_session=private-cookie", csrf: "private-csrf" },
  seed = {
    resources: 10000,
    candidate_bytes: 10000,
    seed_work: 10000,
    done_work: 10000,
    work_bytes: 10000,
    drain_ms: 10,
    elapsed_ms: 10,
  };
const timing = () => ({
  iat: Math.floor(Date.now() / 1000),
  exp: Math.floor(Date.now() / 1000) + 300,
  received_at_ms: Date.now(),
});
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
  yield {
    index: 1,
    planned: 0,
    measured: true,
    requests: [
      {
        method: "POST",
        path: "/rom-studio/api/invoke",
        measurement: "commit",
        body: {
          kind: "load-records",
          id: "load-00001",
          expected: 1,
          idempotency: "same-fixed-key",
          operation: { type: "action", input: { name: "touch", input: false } },
        },
      },
    ],
  };
}
function transport(outcome) {
  let canceled = 0;
  return {
    get canceled() {
      return canceled;
    },
    async acquire(url, options) {
      if (url.endsWith("/live"))
        return new Response(
          new ReadableStream({
            start(c) {
              c.enqueue(new TextEncoder().encode("event: data\ndata: []\n\n"));
            },
            cancel() {
              canceled++;
            },
          }),
          { headers: { "content-type": "text/event-stream" } },
        );
      if (url.endsWith("/capabilities"))
        return new Response(
          '{"protocol_version":1,"inspect":true,"retry":true,"reconcile":true}',
        );
      if (url.endsWith("/list"))
        return new Response(
          '{"protocol_version":1,"records":[],"cursor":null}',
        );
      if (outcome === "denied") return new Response("{}", { status: 403 });
      if (outcome === "unknown" && url.endsWith("/invoke"))
        throw Error("DO-NOT-PERSIST-SECRET");
      return new Response(
        JSON.stringify({
          key: JSON.parse(options.body),
          revision: 1,
          value: {},
        }),
      );
    },
  };
}
function inputs(signal) {
  return {
    session,
    operatorSession: session,
    seed,
    tokenTiming: timing(),
    providerProfile: "exploratory",
    signal,
  };
}
test("composition uses actual executor/scheduler/streams and records numeric unknown outcomes without retry", async () => {
  const io = transport("unknown");
  const result = await runMixedLoad(inputs(), {
    acquire: io.acquire,
    plans: plans(),
  });
  assert.equal(result.traffic.measurements.commit.outcomes.unknown, 1);
  assert.equal(result.traffic.measurements.read.outcomes.success, 1);
  assert.equal(result.http.requests, 2);
  assert.equal(result.operator_http.requests, 4);
  assert.equal(result.streams.readers_drained, true);
  assert.equal(io.canceled, 5);
  assert.equal(result.actual_transport, false);
  assert.equal(result.performance_acceptance, false);
  assert.ok(!JSON.stringify(result).includes("private-cookie"));
  assert.ok(!JSON.stringify(result).includes("DO-NOT-PERSIST"));
});
test("authorization denial stays distinct from success and work cannot imply durable completion", async () => {
  const io = transport("denied");
  const result = await runMixedLoad(inputs(), {
    acquire: io.acquire,
    plans: plans(),
  });
  assert.equal(result.traffic.measurements.read.outcomes.denied, 1);
  assert.equal(result.durable_completion, "awaiting-host-shutdown-proof");
});
test("bad seed admission rejects before HTTP or stream acquisition", async () => {
  let calls = 0;
  await assert.rejects(
    runMixedLoad(
      { ...inputs(), seed: { ...seed, done_work: 9999 } },
      {
        acquire: async () => {
          calls++;
        },
        plans: plans(),
      },
    ),
    /seed/,
  );
  assert.equal(calls, 0);
});
test("external cancellation aborts active HTTP, closes slow reader and retains timeout", async () => {
  const control = new AbortController(),
    io = transport("ok");
  let active = 0;
  const acquire = async (url, options) => {
    if (url.endsWith("/read")) {
      active++;
      setImmediate(() => control.abort());
      return new Promise((_, reject) =>
        options.signal.addEventListener(
          "abort",
          () => {
            active--;
            reject(new DOMException("cancelled", "AbortError"));
          },
          { once: true },
        ),
      );
    }
    return io.acquire(url, options);
  };
  const result = await runMixedLoad(inputs(control.signal), {
    acquire,
    plans: plans(),
  });
  assert.equal(active, 0);
  assert.equal(result.cancelled, true);
  assert.equal(result.traffic.measurements.read.outcomes.timeout, 1);
  assert.equal(result.streams.readers_drained, true);
  assert.equal(io.canceled, 5);
});
test("durable final host proof must be bounded complete and explicitly measured", () => {
  const draft = { performance_acceptance: false };
  assert.throws(
    () =>
      attachMixedHostProof(
        draft,
        { records: 12000, bytes: 100, done: 11999 },
        {
          semantics: "fixture-commit-confirmation-to-claim-confirmation",
          restart_durable: false,
          nanosecond_samples: [1],
          missing: 0,
          rejected: 0,
        },
        { rss_bytes: 1, disk_bytes: 1 },
      ),
    /complete/,
  );
  const result = attachMixedHostProof(
    draft,
    { records: 12000, bytes: 100, done: 12000 },
    {
      semantics: "fixture-commit-confirmation-to-claim-confirmation",
      restart_durable: false,
      nanosecond_samples: [1, 3],
      missing: 0,
      rejected: 0,
    },
    { rss_bytes: 1, disk_bytes: 1 },
  );
  assert.equal(result.host.queue_confirmation.maximum_ns, 3);
  assert.equal(result.durable_completion, "confirmed-final-host-proof");
  assert.equal(result.performance_acceptance, false);
});
test("backpressure and invalid oversized stream frames retain bounded errors and close all readers", async () => {
  const io = transport("ok");
  const acquire = (url, options) =>
    url.endsWith("/live")
      ? Promise.resolve(
          new Response(
            new ReadableStream({
              start(c) {
                c.enqueue(new Uint8Array(65537));
              },
              cancel() {},
            }),
            { headers: { "content-type": "text/event-stream" } },
          ),
        )
      : io.acquire(url, options);
  const result = await runMixedLoad(inputs(), { acquire, plans: plans() });
  assert.equal(result.streams.error_events, 4);
  assert.equal(result.streams.slow, 1);
  assert.equal(result.streams.readers_drained, true);
  assert.equal(result.status, "failed");
});
test("already-cancelled control makes no HTTP calls", async () => {
  const c = new AbortController();
  c.abort();
  let calls = 0;
  const result = await runMixedLoad(inputs(c.signal), {
    acquire: async () => {
      calls++;
    },
    plans: plans(),
  });
  assert.equal(calls, 0);
  assert.equal(result.status, "cancelled");
});

async function localStreamServer(mode, exercise) {
  const { createServer } = await import("node:http");
  const server = createServer((request, response) => {
    if (mode === "stall") return;
    if (mode === "body") {
      response.writeHead(200, { "content-type": "application/json" });
      response.write("{");
      return;
    }
    response.writeHead(200, { "content-type": "text/event-stream" });
    if (mode === "eof") {
      response.end();
      return;
    }
    response.write("event: data\ndata: []\n\n");
    const timer = setInterval(() => response.write(": heartbeat\n\n"), 100);
    response.on("close", () => clearInterval(timer));
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  try {
    return await exercise(`http://127.0.0.1:${server.address().port}/live`);
  } finally {
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }
}
test(
  "real signal-bound stream bodies survive the former five-second header deadline",
  { timeout: 12000 },
  async () => {
    await localStreamServer("stream", async (url) => {
      const io = transport();
      let signals = [];
      function* delayedPlans() {
        yield { ...plans().next().value, planned: 5300 };
      }
      const result = await runMixedLoad(inputs(), {
        acquire: (target, options) => {
          if (!target.endsWith("/live")) return io.acquire(target, options);
          signals.push(options.signal);
          return fetch(url, options);
        },
        plans: delayedPlans(),
      });
      assert.ok(result.elapsed_ms >= 5200);
      assert.equal(signals.length, 5);
      assert.equal(result.streams.error_events, 0);
      assert.equal(result.status, "completed");
      assert.equal(result.streams.readers_drained, true);
    });
  },
);
test(
  "stalled real stream headers expire within the owned header deadline",
  { timeout: 9000 },
  async () => {
    await localStreamServer("stall", async (url) => {
      const io = transport();
      const start = performance.now();
      const result = await runMixedLoad(inputs(), {
        acquire: (target, options) =>
          target.endsWith("/live")
            ? fetch(url, options)
            : io.acquire(target, options),
        plans: plans(),
      });
      assert.equal(result.status, "failed");
      assert.ok(performance.now() - start >= 4900);
      assert.ok(performance.now() - start < 8000);
    });
  },
);
test(
  "external cancellation aborts stalled real stream headers promptly",
  { timeout: 2000 },
  async () => {
    await localStreamServer("stall", async (url) => {
      const io = transport(),
        control = new AbortController();
      const timer = setTimeout(() => control.abort(), 100);
      try {
        const result = await runMixedLoad(inputs(control.signal), {
          acquire: (target, options) =>
            target.endsWith("/live")
              ? fetch(url, options)
              : io.acquire(target, options),
          plans: plans(),
        });
        assert.equal(result.status, "cancelled");
        assert.equal(result.cancelled, true);
      } finally {
        clearTimeout(timer);
      }
    });
  },
);
test("seed-only completed Work cannot prove the full mixed effects", () => {
  assert.throws(
    () =>
      attachMixedHostProof(
        {},
        { records: 10000, bytes: 100, done: 10000 },
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
test("premature real SSE EOF fails mixed traffic", async () => {
  await localStreamServer("eof", async (url) => {
    const io = transport();
    const result = await runMixedLoad(inputs(), {
      acquire: (target, options) =>
        target.endsWith("/live")
          ? fetch(url, options)
          : io.acquire(target, options),
      plans: plans(),
    });
    assert.equal(result.status, "failed");
    assert.equal(result.streams.error_events, 4);
    assert.equal(result.streams.readers_drained, true);
  });
});
test(
  "ordinary HTTP still times out while consuming a stalled response body",
  { timeout: 9000 },
  async () => {
    await localStreamServer("body", async (url) => {
      const io = transport();
      const started = performance.now();
      const result = await runMixedLoad(inputs(), {
        acquire: (target, options) =>
          target.endsWith("/read")
            ? fetch(url, options)
            : io.acquire(target, options),
        plans: [plans().next().value],
      });
      assert.equal(result.status, "failed");
      assert.equal(result.traffic.measurements.read.outcomes.timeout, 1);
      assert.ok(performance.now() - started >= 4900);
      assert.ok(performance.now() - started < 8000);
    });
  },
);
