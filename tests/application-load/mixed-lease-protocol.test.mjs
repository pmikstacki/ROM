import test from "node:test";
import assert from "node:assert/strict";
import {
  requireLeaseStreamProof,
  leaseMixedManifest,
} from "./mixed-lease-protocol.mjs";
function proof() {
  return {
    schema: "rom-load-lease-streams-v2",
    normal: 4,
    slow: 1,
    slow_reads: 0,
    bytes: 12345,
    data_events: 20,
    error_events: 0,
    error_classes: {},
    expected_expiries: 16,
    terminal_eofs: 16,
    reacquisition_attempts: 16,
    recoveries: 16,
    fresh_snapshots: 16,
    initial_snapshots: 4,
    abandoned_recoveries: 0,
    cancelled_recoveries: 0,
    refused: 0,
    session_bytes: 4096,
    recovery_latency_ms: Array(16).fill(5),
    observation_gap_ms: Array(16).fill(6),
    gap_semantics: "terminal-eof-to-first-fresh-snapshot",
    additional_http: { session_requests: 17, live_requests: 21 },
    maximum_bytes: 4194304,
    maximum_recoveries_per_reader: 5,
    maximum_ms: 150000,
    deadline_reached: false,
    readers_drained: true,
  };
}
test("complete versioned lease proof preserves measured evidence", () => {
  const p = proof(),
    before = structuredClone(p);
  assert.equal(requireLeaseStreamProof(p), p);
  assert.deepEqual(p, before);
});
for (const [name, change] of [
  [
    "v1 result",
    (p) => {
      p.schema = "raw-v1";
    },
  ],
  [
    "no expected expiry",
    (p) => {
      p.expected_expiries = 0;
    },
  ],
  [
    "missing EOF",
    (p) => {
      p.terminal_eofs--;
    },
  ],
  [
    "no fresh snapshot",
    (p) => {
      p.fresh_snapshots--;
    },
  ],
  [
    "missing recovery",
    (p) => {
      p.recoveries--;
    },
  ],
  [
    "extra acquisition attempt",
    (p) => {
      p.reacquisition_attempts++;
    },
  ],
  [
    "cancelled recovery",
    (p) => {
      p.cancelled_recoveries = 1;
    },
  ],
  [
    "abandoned recovery",
    (p) => {
      p.abandoned_recoveries = 1;
    },
  ],
  [
    "hidden denial",
    (p) => {
      p.error_events = 1;
    },
  ],
  [
    "hidden error class",
    (p) => {
      p.error_classes = { denied: 1 };
    },
  ],
  [
    "wrong reader counts",
    (p) => {
      p.normal = 3;
    },
  ],
  [
    "missing initial snapshot",
    (p) => {
      p.initial_snapshots = 3;
    },
  ],
  [
    "slow body consumed",
    (p) => {
      p.slow_reads = 1;
    },
  ],
  [
    "global byte cap",
    (p) => {
      p.bytes = 4194305;
    },
  ],
  [
    "session body cap",
    (p) => {
      p.session_bytes = 344065;
    },
  ],
  [
    "changed retry budget",
    (p) => {
      p.maximum_recoveries_per_reader = 6;
    },
  ],
  [
    "exhausted retries",
    (p) => {
      p.expected_expiries = 21;
    },
  ],
  [
    "incorrect additional HTTP",
    (p) => {
      p.additional_http.session_requests = 1;
    },
  ],
  [
    "incorrect live HTTP",
    (p) => {
      p.additional_http.live_requests = 5;
    },
  ],
  [
    "unbounded latency",
    (p) => {
      p.recovery_latency_ms[0] = 10001;
    },
  ],
  [
    "negative gap",
    (p) => {
      p.observation_gap_ms[0] = -1;
    },
  ],
  [
    "wrong latency cardinality",
    (p) => {
      p.recovery_latency_ms.pop();
    },
  ],
  [
    "nonfinite count",
    (p) => {
      p.recoveries = NaN;
    },
  ],
  [
    "reader undrained",
    (p) => {
      p.readers_drained = false;
    },
  ],
  [
    "deadline reached",
    (p) => {
      p.deadline_reached = true;
    },
  ],
  [
    "unknown field",
    (p) => {
      p.unsupported = true;
    },
  ],
])
  test(`lease proof rejects ${name}`, () => {
    const p = proof();
    change(p);
    assert.throws(() => requireLeaseStreamProof(p), /lease/);
  });
const manifest = () => ({
  mode: "mixed",
  adapter: "sqlite",
  evidence_parent: "/var/tmp/run/evidence",
  seed_result_path: "/root/ROM/.superpowers/seed.json",
  seed_audit_path: "/root/ROM/.superpowers/audit.json",
  execution_build_identity: "/root/ROM/.superpowers/build.json",
  session_path: "/var/tmp/run/session.json",
  token_timing_path: "/var/tmp/run/timing.json",
  provider_profile: "exploratory",
});
test("explicit v2 selection preserves original closed manifest values", () => {
  const input = manifest(),
    before = structuredClone(input),
    selected = leaseMixedManifest(input);
  assert.equal(selected.stream_version, "lease-v2");
  delete selected.stream_version;
  assert.deepEqual(selected, before);
  assert.deepEqual(input, before);
});
for (const [name, change] of [
  [
    "v1 override",
    (p) => {
      p.stream_version = "raw-v1";
    },
  ],
  [
    "unknown selector",
    (p) => {
      p.stream_version = "future";
    },
  ],
  [
    "extra field",
    (p) => {
      p.unsupported = true;
    },
  ],
  [
    "wrong mode",
    (p) => {
      p.mode = "seed";
    },
  ],
  [
    "relative path",
    (p) => {
      p.session_path = "relative";
    },
  ],
  [
    "traversal",
    (p) => {
      p.session_path = "/var/tmp/run/../secret";
    },
  ],
  [
    "wrong adapter",
    (p) => {
      p.adapter = "unknown";
    },
  ],
  [
    "missing input",
    (p) => {
      delete p.seed_result_path;
    },
  ],
])
  test(`v2 selection rejects ${name}`, () => {
    const p = manifest();
    change(p);
    assert.throws(() => leaseMixedManifest(p), /lease/);
  });
