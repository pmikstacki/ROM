import { performance } from "node:perf_hooks";
import { SchedulerDiagnostics } from "./scheduler-diagnostics.mjs";
import { Admission } from "./admission.mjs";
import { Measurements } from "./measurements.mjs";
// Scheduled arrivals include generator delay. No request payload enters samples.
export async function runSchedule(plans, execute, limits, deadlineMs = 150000) {
  if (
    typeof execute !== "function" ||
    !Number.isSafeInteger(deadlineMs) ||
    deadlineMs < 1 ||
    deadlineMs > 150000
  )
    throw Error("finite scheduled run");
  const admission = new Admission(limits),
    measurements = new Measurements(),
    iterator = plans[Symbol.iterator]();
  const started = performance.now(),
    now = () => performance.now() - started,
    controller = new AbortController();
  let deadlineReached = false;
  const timer = setTimeout(() => {
    deadlineReached = true;
    controller.abort();
  }, deadlineMs);
  const diagnostics = new SchedulerDiagnostics(limits.active);
  const active = new Set(),
    lookup = new Map();
  let next = iterator.next();
  function start(value) {
    const group = lookup.get(value.index);
    lookup.delete(value.index);
    diagnostics.dispatch(value.index, now());
    const promise = (async () => {
      try {
        let requestIndex = 0;
        for (const request of group.requests) {
          if (controller.signal.aborted) {
            diagnostics.cancelled(group.requests.length - requestIndex);
            break;
          }
          requestIndex++;
          const dispatched = Math.max(group.planned, now());
          let outcome;
          try {
            outcome = await execute(request, controller.signal);
          } catch {
            outcome = "failure";
          }
          if (group.measured)
            measurements.record({
              operation: request.measurement,
              planned: group.planned,
              dispatched,
              completed: Math.max(dispatched, now()),
              outcome,
            });
          if (outcome !== "success") {
            diagnostics.nonSuccess(group.requests.length - requestIndex);
            break;
          }
        }
      } finally {
        admission.finish(value.index);
      }
    })();
    active.add(promise);
    promise.finally(() => active.delete(promise));
  }
  function pump() {
    let value;
    while (!controller.signal.aborted && (value = admission.take()))
      start(value);
  }
  try {
    while (!next.done || admission.stats().pending || active.size) {
      let dueBatch = 0;
      while (
        !next.done &&
        next.value.planned <= now() &&
        !controller.signal.aborted
      ) {
        const group = next.value;
        dueBatch++;
        if (
          !Array.isArray(group.requests) ||
          group.requests.length < 1 ||
          group.requests.length > 3 ||
          typeof group.measured !== "boolean"
        )
          throw Error("bounded request group");
        pump();
        const offered = now();
        const accepted = admission.offer({
          index: group.index,
          planned: group.planned,
        });
        diagnostics.offer(
          group.index,
          group.category,
          group.planned,
          offered,
          accepted,
          admission.stats().active,
        );
        if (accepted) {
          lookup.set(group.index, group);
          pump();
        }
        next = iterator.next();
      }
      diagnostics.dueBatch(dueBatch);
      if (controller.signal.aborted) break;
      pump();
      if (next.done && !active.size && !admission.stats().pending) break;
      const wait = next.done
        ? deadlineMs - now()
        : Math.max(0, next.value.planned - now());
      let pause;
      await Promise.race([
        ...active,
        new Promise((resolve) => {
          pause = setTimeout(
            resolve,
            Math.max(1, Math.min(wait, deadlineMs - now())),
          );
        }),
      ]);
      clearTimeout(pause);
    }
  } finally {
    clearTimeout(timer);
    controller.abort();
    await Promise.all(active);
  }
  return {
    admission: admission.stats(),
    diagnostics: diagnostics.snapshot(),
    measurements: measurements.summary(),
    deadline_reached: deadlineReached,
    elapsed_ms: now(),
  };
}
