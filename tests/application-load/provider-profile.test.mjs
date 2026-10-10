import test from "node:test";
import assert from "node:assert/strict";
import { providerWindow } from "./provider-profile.mjs";

test("existing defaults stay finite and the load relay fits strict supervision", () => {
  assert.deepEqual(providerWindow([]), {
    ready_seconds: 300,
    container_seconds: 600,
    relay_milliseconds: 540000,
  });
  const long = providerWindow(["--load-window"]);
  assert.equal(long.ready_seconds, 1200);
  assert.equal(long.container_seconds, 1500);
  assert.equal(long.relay_milliseconds, 1200000);
  assert.equal(long.ready_seconds * 1000, long.relay_milliseconds);
  assert.ok(long.relay_milliseconds < long.container_seconds * 1000);
});

test("unknown arguments cannot widen or disable provider lifetimes", () => {
  for (const args of [
    ["--load-window", "--load-window"],
    ["--forever"],
    ["--load-window", "--timeout=0"],
    ["--timeout=999999999"],
    "--load-window",
  ]) {
    assert.throws(() => providerWindow(args));
  }
});
