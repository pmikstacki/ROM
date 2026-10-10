import test from "node:test";
import assert from "node:assert/strict";
import { providerWindow } from "./window-profile.mjs";
import { ProcessRegistry } from "./supervision.mjs";

test("both maintained relay profiles pass actual supervisor admission without widening its cap", () => {
  const registry = new ProcessRegistry();
  for (const args of [[], ["--load-window"]]) {
    assert.doesNotThrow(() => registry.admit({
      timeoutMs: providerWindow(args).relay_milliseconds,
      graceMs: 1000,
      drainMs: 1000,
      outputBytes: 65536,
    }));
  }
  assert.throws(() => registry.admit({ timeoutMs: 1200001 }), /invalid supervision limits/);
});

test("existing defaults remain finite and the explicit load profile uses the supervisor cap", () => {
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

test("nominal ready limit cannot erase relay startup or cleanup time", () => {
  const record = readyWindow(providerWindow(["--load-window"]), {
    container_started_unix_ms: 1000,
    relay_started_unix_ms: 1000,
    health_finished_unix_ms: 181000,
  });
  assert.equal(record.admission_remaining_ms, 975000);
  assert.equal(requireWorkloadWindow(record, 840000, 181000), 975000);
  assert.throws(() => requireWorkloadWindow(record, 840000, 321000));
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

import { providerLifetimeSeconds } from "./window-profile.mjs";
test("container launch accepts only maintained finite lifetimes", () => {
  assert.equal(providerLifetimeSeconds(), 600);
  assert.equal(providerLifetimeSeconds(1500), 1500);
  for (const value of [0, -1, 1501, Infinity, "1500", null]) {
    assert.throws(() => providerLifetimeSeconds(value));
  }
});

import { readyWindow, requireWorkloadWindow } from "./window-profile.mjs";
test("startup consumes admission time and cleanup reserve precedes actual lifetime deadlines", () => {
  const record = readyWindow(providerWindow(["--load-window"]), {
    container_started_unix_ms: 1000,
    relay_started_unix_ms: 301000,
    health_finished_unix_ms: 601000,
  });
  assert.equal(record.container_deadline_unix_ms, 1501000);
  assert.equal(record.ready_deadline_unix_ms, 1456000);
  assert.equal(record.admission_remaining_ms, 855000);
  assert.equal(requireWorkloadWindow(record, 840000, 601000), 855000);
  assert.throws(() => requireWorkloadWindow(record, 840000, 621000));
  assert.throws(() => requireWorkloadWindow(record, 0, 601000));
});

test("expired lifetimes and malformed timing do not produce an admitted ready window", () => {
  for (const times of [
    {
      container_started_unix_ms: 0,
      relay_started_unix_ms: 100,
      health_finished_unix_ms: 600000,
    },
    {
      container_started_unix_ms: 10,
      relay_started_unix_ms: 0,
      health_finished_unix_ms: 20,
    },
    {
      container_started_unix_ms: 0,
      relay_started_unix_ms: 1,
      health_finished_unix_ms: NaN,
    },
  ])
    assert.throws(() => readyWindow(providerWindow([]), times));
});
