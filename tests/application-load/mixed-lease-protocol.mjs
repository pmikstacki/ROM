const record = (value) =>
  value !== null && typeof value === "object" && !Array.isArray(value);
const integer = (value, max) =>
  Number.isSafeInteger(value) && value >= 0 && value <= max;
const refuse = () => {
  throw Error("complete bounded lease protocol required");
};
const fields = [
  "schema",
  "normal",
  "slow",
  "slow_reads",
  "bytes",
  "data_events",
  "error_events",
  "error_classes",
  "expected_expiries",
  "terminal_eofs",
  "reacquisition_attempts",
  "recoveries",
  "fresh_snapshots",
  "initial_snapshots",
  "abandoned_recoveries",
  "cancelled_recoveries",
  "refused",
  "session_bytes",
  "recovery_latency_ms",
  "observation_gap_ms",
  "gap_semantics",
  "additional_http",
  "maximum_bytes",
  "maximum_recoveries_per_reader",
  "maximum_ms",
  "deadline_reached",
  "readers_drained",
];
export function requireLeaseStreamProof(p) {
  if (
    !record(p) ||
    Object.keys(p).length !== fields.length ||
    fields.some((name) => !Object.hasOwn(p, name)) ||
    p.schema !== "rom-load-lease-streams-v2" ||
    p.normal !== 4 ||
    p.slow !== 1 ||
    p.slow_reads !== 0 ||
    p.initial_snapshots !== 4 ||
    p.error_events !== 0 ||
    !record(p.error_classes) ||
    Object.keys(p.error_classes).length !== 0 ||
    p.refused !== 0 ||
    p.abandoned_recoveries !== 0 ||
    p.cancelled_recoveries !== 0 ||
    p.deadline_reached !== false ||
    p.readers_drained !== true ||
    p.maximum_bytes !== 4194304 ||
    p.maximum_recoveries_per_reader !== 5 ||
    p.maximum_ms !== 150000 ||
    p.gap_semantics !== "terminal-eof-to-first-fresh-snapshot"
  )
    refuse();
  const n = p.recoveries;
  if (
    !integer(n, 20) ||
    n < 4 ||
    p.expected_expiries !== n ||
    p.terminal_eofs !== n ||
    p.reacquisition_attempts !== n ||
    p.fresh_snapshots !== n ||
    !integer(p.bytes, 4194304) ||
    p.bytes === 0 ||
    !integer(p.session_bytes, 344064) ||
    p.session_bytes === 0 ||
    !integer(p.data_events, 4194304) ||
    p.data_events < 4 + n
  )
    refuse();
  if (
    !record(p.additional_http) ||
    Object.keys(p.additional_http).length !== 2 ||
    p.additional_http.session_requests !== 1 + n ||
    p.additional_http.live_requests !== 5 + n
  )
    refuse();
  for (const values of [p.recovery_latency_ms, p.observation_gap_ms])
    if (
      !Array.isArray(values) ||
      values.length !== n ||
      values.some(
        (value) =>
          typeof value !== "number" ||
          !Number.isFinite(value) ||
          value < 0 ||
          value > 10000,
      )
    )
      refuse();
  if (
    p.observation_gap_ms.some(
      (value, index) => value < p.recovery_latency_ms[index],
    )
  )
    refuse();
  return p;
}
const required = [
  "mode",
  "adapter",
  "evidence_parent",
  "seed_result_path",
  "seed_audit_path",
  "execution_build_identity",
  "session_path",
  "token_timing_path",
  "provider_profile",
];
export function leaseMixedManifest(value) {
  if (
    !record(value) ||
    required.some((name) => !Object.hasOwn(value, name)) ||
    Object.keys(value).some(
      (name) => !required.includes(name) && name !== "stream_version",
    ) ||
    value.mode !== "mixed" ||
    !["sqlite", "redb"].includes(value.adapter) ||
    value.provider_profile !== "exploratory" ||
    (Object.hasOwn(value, "stream_version") &&
      value.stream_version !== "lease-v2")
  )
    refuse();
  for (const name of [
    "evidence_parent",
    "seed_result_path",
    "seed_audit_path",
    "execution_build_identity",
    "session_path",
    "token_timing_path",
  ]) {
    const path = value[name];
    if (
      typeof path !== "string" ||
      path.length > 4096 ||
      !/^\/[A-Za-z0-9/_.-]+$/.test(path) ||
      path
        .split("/")
        .slice(1)
        .some((part) => part === "" || part === "." || part === "..")
    )
      refuse();
  }
  return { ...value, stream_version: "lease-v2" };
}
