import test from "node:test";
import assert from "node:assert/strict";
import { getEventListeners } from "node:events";
import { withDeadline, signalBody } from "./http-lifetime.mjs";

test("owned body deadline cancels a stalled reader and releases its lock", async () => {
  let reason;
  const body = new ReadableStream({ cancel(value) { reason = value; } });
  await assert.rejects(withDeadline(20, [], async (signal) => {
    for await (const chunk of signalBody(body, signal)) assert.fail("unexpected chunk");
  }), { name: "TimeoutError" });
  assert.equal(reason.name, "TimeoutError");
  assert.equal(body.locked, false);
});

test("external cancellation retains its reason and removes forwarding listeners", async () => {
  const control = new AbortController(), reason = Error("fixture cancellation");
  const body = new ReadableStream();
  const pending = withDeadline(5000, [control.signal], async (signal) => {
    for await (const chunk of signalBody(body, signal)) assert.fail("unexpected chunk");
  });
  control.abort(reason);
  await assert.rejects(pending, (error) => error === reason);
  assert.equal(getEventListeners(control.signal, "abort").length, 0);
  assert.equal(body.locked, false);
});

test("completed body work clears its deadline without a late abort", async () => {
  let aborted = 0;
  await withDeadline(10, [], async (signal) => {
    signal.addEventListener("abort", () => aborted++);
    const body = new ReadableStream({ start(controller) { controller.enqueue("x"); controller.close(); } });
    const chunks = [];
    for await (const chunk of signalBody(body, signal)) chunks.push(chunk);
    assert.deepEqual(chunks, ["x"]);
    assert.equal(body.locked, false);
  });
  await new Promise((resolve) => setTimeout(resolve, 30));
  assert.equal(aborted, 0);
});
