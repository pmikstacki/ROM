// Execute the maintained mixed HTTP profile after the separately verified seed.
import { performance } from "node:perf_hooks";
import {
  readFileSync,
  lstatSync,
  realpathSync,
  mkdirSync,
  openSync,
  writeSync,
  fsyncSync,
  closeSync,
} from "node:fs";
import { randomBytes } from "node:crypto";
import { fileURLToPath } from "node:url";
import { httpExecutor } from "./http-executor.mjs";
import { runSchedule } from "./scheduler.mjs";
import { groups } from "./workload-plan.mjs";
import { openLeaseStreams } from "./lease-streams.mjs";
import { openLoadStreams } from "./streams.mjs";
import { Measurements } from "./measurements.mjs";
import {
  exploratoryProfile,
  requireSeedProof,
  requireLedgerProof,
} from "./profile.mjs";
import { requireTokenWindow } from "./provider-validity.mjs";
import { captureWorkParserSources } from "./work-parser.mjs";
import { verifyLoadSources } from "./source-witness.mjs";
import { verifyPreservedBinary } from "../identity/provider/build-witness.mjs";
const origin = "https://127.0.0.1:44389";
function admitSeed(seed, profile) {
  const { elapsed_ms, ...proof } = seed ?? {};
  requireSeedProof(profile, proof);
  if (
    !Number.isSafeInteger(elapsed_ms) ||
    elapsed_ms < 0 ||
    elapsed_ms > 120000
  )
    throw Error("actual seed deadline proof");
}
function* untilAborted(plans, signal) {
  for (const group of plans) {
    if (signal.aborted) return;
    yield group;
  }
}
async function closeStreams(streams) {
  let timer;
  try {
    return await Promise.race([
      streams.close(),
      new Promise((resolve) => {
        timer = setTimeout(
          () =>
            resolve({ readers_drained: false, cleanup_deadline_reached: true }),
          5000,
        );
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}
export function runMixedLoad(input, dependencies = {}) {
  return runMixedCore(input, dependencies, "raw-v1");
}
export function runMixedLeaseLoad(input, dependencies = {}) {
  return runMixedCore(input, dependencies, "lease-v2");
}
export async function runSelectedMixedLoad(version, input, dependencies = {}) {
  if (!["raw-v1", "lease-v2"].includes(version))
    throw Error("closed mixed stream version");
  return version === "raw-v1"
    ? runMixedLoad(input, dependencies)
    : runMixedLeaseLoad(input, dependencies);
}
async function runMixedCore(input, dependencies, version) {
  const profile = exploratoryProfile();
  admitSeed(input?.seed, profile);
  const tokenWindow = requireTokenWindow(
    input.tokenTiming,
    Date.now(),
    150000,
    75000,
    input.providerProfile,
  );
  if (input.signal !== undefined && !(input.signal instanceof AbortSignal))
    throw Error("bounded external cancellation signal");
  if (Object.keys(dependencies).some((k) => !["acquire", "plans"].includes(k)))
    throw Error("closed mixed driver dependencies");
  const stop = new AbortController(),
    signal = AbortSignal.any([
      stop.signal,
      AbortSignal.timeout(180000),
      ...(input.signal ? [input.signal] : []),
    ]);
  const rawAcquire = dependencies.acquire ?? fetch;
  const acquire = async (url, options) => {
    if (url !== "https://127.0.0.1:44389/rom-studio/api/live")
      return await rawAcquire(url, options);
    const headers = new AbortController();
    const timer =
      url === "https://127.0.0.1:44389/rom-studio/api/live"
        ? setTimeout(
            () =>
              headers.abort(
                new DOMException("Stream headers timed out", "TimeoutError"),
              ),
            5000,
          )
        : undefined;
    try {
      return await rawAcquire(url, {
        ...options,
        signal: AbortSignal.any([
          options.signal,
          signal,
          timer === undefined ? AbortSignal.timeout(5000) : headers.signal,
        ]),
      });
    } finally {
      if (timer !== undefined) clearTimeout(timer);
    }
  };
  const http = httpExecutor(input.session, acquire),
    operator = httpExecutor(input.operatorSession, acquire),
    work = new Measurements(4),
    start = performance.now();
  const result = {
    schema:
      version === "raw-v1"
        ? "rom-application-mixed-driver-v1"
        : "rom-application-mixed-driver-v2",
    status: "running",
    actual_transport:
      dependencies.acquire === undefined && dependencies.plans === undefined,
    performance_acceptance: false,
    profile,
    token_window: tokenWindow,
    durable_completion: "awaiting-host-shutdown-proof",
    client_rss_before_bytes: process.memoryUsage().rss,
    parser_sources: captureWorkParserSources(),
    maximum_http_requests: 13284,
    maximum_http_response_bytes: 64 * 1024 ** 2 + 4 * 65536,
    maximum_stream_bytes: 4 * 1024 ** 2,
  };
  let streams;
  const outcomes = Object.fromEntries(
    ["success", "denied", "overloaded", "unknown", "timeout", "failure"].map(
      (label) => [label, 0],
    ),
  );
  async function inspect() {
    for (const [route, body] of [
      ["capabilities", {}],
      ["list", { limit: 50, definition: "load-observation" }],
    ]) {
      if (signal.aborted) break;
      const dispatched = performance.now() - start;
      const outcome = await operator.execute(
        {
          method: "POST",
          path: `/rom-studio/api/work/${route}`,
          body,
          measurement: "work",
          operator: true,
          ...(route === "capabilities"
            ? { expected_capability: "inspect" }
            : {}),
        },
        signal,
      );
      work.record({
        operation: "work",
        planned: dispatched,
        dispatched,
        completed: performance.now() - start,
        outcome,
      });
      if (outcome !== "success") return false;
    }
    return !signal.aborted;
  }
  try {
    if (!(await inspect())) throw Error("operator inspection unavailable");
    streams = await (version === "raw-v1" ? openLoadStreams : openLeaseStreams)(
      input.session,
      { normal: 4, slow: 1 },
      acquire,
      ...(version === "lease-v2" ? [{ signal }] : []),
    );
    result.traffic = await runSchedule(
      untilAborted(dependencies.plans ?? groups(), signal),
      async (request, scheduled) => {
        const outcome = await http.execute(
          request,
          AbortSignal.any([signal, scheduled]),
        );
        outcomes[outcome]++;
        return outcome;
      },
      { active: 32, pending: 64, scheduled: 12000 },
      150000,
    );
    if (!signal.aborted && !(await inspect()))
      throw Error("operator inspection unavailable");
    result.status = signal.aborted ? "cancelled" : "completed";
  } catch {
    result.status = signal.aborted ? "cancelled" : "failed";
    result.failure = "mixed-traffic-or-inspection";
  } finally {
    if (streams) result.streams = await closeStreams(streams);
    else result.streams = { readers_drained: true, opened: false };
    const wasAborted = signal.aborted;
    stop.abort();
    result.http = http.counts();
    result.http_outcomes = outcomes;
    result.operator_http = operator.counts();
    result.work_inspection = work.summary().work;
    result.cancelled = input.signal?.aborted === true;
    result.deadline_reached = wasAborted && !result.cancelled;
    result.elapsed_ms = performance.now() - start;
    result.client_rss_after_bytes = process.memoryUsage().rss;
    if (
      !result.streams.readers_drained ||
      (version === "lease-v2" &&
        !wasAborted &&
        (result.streams.initial_snapshots !== 4 ||
          result.streams.deadline_reached ||
          result.streams.cancelled_recoveries !== 0 ||
          result.streams.abandoned_recoveries !== 0 ||
          result.streams.expected_expiries !== result.streams.terminal_eofs ||
          result.streams.expected_expiries !== result.streams.recoveries ||
          result.streams.recoveries !== result.streams.fresh_snapshots)) ||
      (result.actual_transport &&
        (!result.traffic ||
          result.traffic.deadline_reached ||
          result.traffic.admission.scheduled !== 12000 ||
          result.traffic.admission.rejected_scheduling !== 0 ||
          result.http.requests !== 13280)) ||
      (!wasAborted && result.streams.error_events > 0) ||
      (!wasAborted &&
        Object.entries(outcomes).some(
          ([label, count]) => label !== "success" && count > 0,
        ))
    )
      result.status = "failed";
  }
  return result;
}
export function attachMixedHostProof(result, finalWork, queue, nativeMetrics) {
  const profile = exploratoryProfile();
  requireLedgerProof(profile, {
    records: finalWork?.records,
    bytes: finalWork?.bytes,
  });
  if (
    !Number.isSafeInteger(finalWork.done) ||
    finalWork.done !== finalWork.records ||
    finalWork.records !==
      profile.seed_resources +
        profile.traffic.mutations +
        profile.traffic.notification_intents
  )
    throw Error("complete durable host proof required");
  if (
    queue?.semantics !== "fixture-commit-confirmation-to-claim-confirmation" ||
    queue.restart_durable !== false ||
    !Array.isArray(queue.nanosecond_samples) ||
    queue.nanosecond_samples.length > 12000 ||
    ![queue.missing, queue.rejected, ...queue.nanosecond_samples].every(
      (n) => Number.isSafeInteger(n) && n >= 0,
    )
  )
    throw Error("bounded queue confirmation proof");
  if (
    !nativeMetrics ||
    Object.keys(nativeMetrics).length !== 2 ||
    ![nativeMetrics.rss_bytes, nativeMetrics.disk_bytes].every(
      (n) => Number.isSafeInteger(n) && n >= 0,
    )
  )
    throw Error("actual host RSS/disk measurements required");
  let total = 0,
    maximum = 0;
  for (const value of queue.nanosecond_samples) {
    total += value;
    maximum = Math.max(maximum, value);
    if (!Number.isSafeInteger(total)) throw Error("queue measurement overflow");
  }
  return {
    ...result,
    durable_completion: "confirmed-final-host-proof",
    performance_acceptance: false,
    host: {
      final_work: {
        records: finalWork.records,
        bytes: finalWork.bytes,
        done: finalWork.done,
      },
      queue_confirmation: {
        semantics: queue.semantics,
        restart_durable: false,
        count: queue.nanosecond_samples.length,
        total_ns: total,
        maximum_ns: maximum,
        missing: queue.missing,
        rejected: queue.rejected,
      },
      native_metrics: { ...nativeMetrics },
    },
  };
}
function privateJson(path, maximum = 65536) {
  const s = lstatSync(path);
  if (
    !s.isFile() ||
    s.isSymbolicLink() ||
    s.nlink !== 1 ||
    s.uid !== process.getuid() ||
    realpathSync(path) !== path ||
    (s.mode & 0o777) !== 0o600 ||
    s.size > maximum
  )
    throw Error("bounded private mixed input");
  return JSON.parse(readFileSync(path, "utf8"));
}
function writeResult(path, value) {
  const bytes = Buffer.from(JSON.stringify(value, null, 2) + "\n");
  if (bytes.length > 65536) throw Error("bounded numeric mixed evidence");
  const fd = openSync(path, "wx", 0o600);
  try {
    writeSync(fd, bytes);
    fsyncSync(fd);
  } finally {
    closeSync(fd);
  }
}
async function cli() {
  if (process.argv.length !== 3)
    throw Error("one private mixed manifest required");
  const cfg = privateJson(process.argv[2]);
  if (
    cfg.mode !== "mixed" ||
    !["sqlite", "redb"].includes(cfg.adapter) ||
    typeof cfg.evidence_parent !== "string" ||
    !/^\/var\/tmp\/[\w/.-]+$/.test(cfg.evidence_parent)
  )
    throw Error("closed admitted mixed manifest");
  const seed = privateJson(cfg.seed_result_path),
    build = privateJson(seed.build_identity, 512 * 1024),
    sessions = privateJson(cfg.session_path),
    timing = privateJson(cfg.token_timing_path);
  if (
    seed.status !== "passed" ||
    seed.adapter !== cfg.adapter ||
    seed.source_fence_before !== true ||
    seed.source_fence_after !== true ||
    build.plan?.diagnostic === true
  )
    throw Error("matching optimized completed seed required");
  verifyLoadSources(build.sources);
  verifyPreservedBinary(build.binary);
  const parent = lstatSync(cfg.evidence_parent);
  if (
    !parent.isDirectory() ||
    parent.isSymbolicLink() ||
    parent.uid !== process.getuid() ||
    realpathSync(cfg.evidence_parent) !== cfg.evidence_parent ||
    (parent.mode & 0o077) !== 0
  )
    throw Error("private evidence parent required");
  const directory = `${cfg.evidence_parent}/mixed-${randomBytes(12).toString("hex")}`;
  mkdirSync(directory, { mode: 0o700 });
  const controller = new AbortController(),
    cancel = () => controller.abort();
  process.on("SIGINT", cancel);
  process.on("SIGTERM", cancel);
  let result;
  try {
    result = await runSelectedMixedLoad(cfg.stream_version ?? "raw-v1", {
      session: sessions.session,
      operatorSession: sessions.operator_session,
      seed: seed.seed,
      tokenTiming: timing,
      providerProfile: cfg.provider_profile,
      signal: controller.signal,
    });
    result.adapter = cfg.adapter;
    result.build_identity = seed.build_identity;
    result.source_fence_before = true;
    verifyLoadSources(build.sources);
    verifyPreservedBinary(build.binary);
    result.source_fence_after = true;
  } catch {
    result = {
      ...(result ?? {
        schema:
          cfg.stream_version === "lease-v2"
            ? "rom-application-mixed-driver-v2"
            : "rom-application-mixed-driver-v1",
      }),
      status: "admission-or-source-failed",
      source_fence_after: false,
      performance_acceptance: false,
    };
  } finally {
    process.off("SIGINT", cancel);
    process.off("SIGTERM", cancel);
    writeResult(
      `${directory}/result.json`,
      result ?? { status: "failed", performance_acceptance: false },
    );
  }
  process.exitCode = result.status === "completed" ? 0 : 1;
  console.log(
    JSON.stringify({
      status: result.status,
      evidence: `${directory}/result.json`,
      performance_acceptance: false,
    }),
  );
}
if (
  process.argv[1] &&
  fileURLToPath(import.meta.url) === realpathSync(process.argv[1])
)
  await cli();
