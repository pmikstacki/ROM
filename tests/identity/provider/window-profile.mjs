// Closed synthetic-provider windows. Production configuration does not use this helper.
export function providerWindow(args) {
  if (
    !Array.isArray(args) ||
    args.length > 1 ||
    (args.length === 1 && args[0] !== "--load-window")
  )
    throw Error("closed provider window profile");
  return Object.freeze(
    args.length
      ? {
          ready_seconds: 1200,
          container_seconds: 1500,
          relay_milliseconds: 1200000,
        }
      : {
          ready_seconds: 300,
          container_seconds: 600,
          relay_milliseconds: 540000,
        },
  );
}

export function providerLifetimeSeconds(value = 600) {
  if (value !== 600 && value !== 1500)
    throw Error("closed provider container lifetime");
  return value;
}

const cleanupReserve = 45000;
export function readyWindow(profile, timings) {
  const known = [providerWindow([]), providerWindow(["--load-window"])];
  if (
    !profile ||
    Object.keys(profile).length !== 3 ||
    !known.some((p) => Object.keys(p).every((k) => p[k] === profile[k]))
  )
    throw Error("closed provider window profile");
  const keys = [
    "container_started_unix_ms",
    "relay_started_unix_ms",
    "health_finished_unix_ms",
  ];
  if (
    !timings ||
    Object.keys(timings).length !== keys.length ||
    keys.some((k) => !Number.isSafeInteger(timings[k]) || timings[k] < 0) ||
    timings.container_started_unix_ms > timings.relay_started_unix_ms ||
    timings.relay_started_unix_ms > timings.health_finished_unix_ms
  )
    throw Error("provider admission timing");
  const containerDeadline =
    timings.container_started_unix_ms + profile.container_seconds * 1000;
  const relayDeadline =
    timings.relay_started_unix_ms + profile.relay_milliseconds;
  const readyDeadline = Math.min(
    timings.health_finished_unix_ms + profile.ready_seconds * 1000,
    containerDeadline - cleanupReserve,
    relayDeadline - cleanupReserve,
  );
  const remaining = readyDeadline - timings.health_finished_unix_ms;
  if (!Number.isSafeInteger(readyDeadline) || remaining <= 0)
    throw Error("provider admission window exhausted");
  return Object.freeze({
    profile: Object.freeze({ ...profile }),
    timings: Object.freeze({ ...timings }),
    container_deadline_unix_ms: containerDeadline,
    relay_deadline_unix_ms: relayDeadline,
    cleanup_reserve_ms: cleanupReserve,
    ready_deadline_unix_ms: readyDeadline,
    admission_remaining_ms: remaining,
  });
}
export function requireWorkloadWindow(record, durationMs, now = Date.now()) {
  const checked = readyWindow(record?.profile, record?.timings);
  if (
    Object.keys(record).length !== Object.keys(checked).length ||
    Object.keys(checked).some(
      (k) => typeof checked[k] === "number" && record[k] !== checked[k],
    )
  )
    throw Error("provider admission evidence");
  if (
    !Number.isSafeInteger(durationMs) ||
    durationMs < 1 ||
    durationMs > 840000 ||
    !Number.isSafeInteger(now) ||
    now < checked.timings.health_finished_unix_ms
  )
    throw Error("finite workload window required");
  const remaining = checked.ready_deadline_unix_ms - now;
  if (remaining < durationMs)
    throw Error("insufficient provider window before workload");
  return remaining;
}
