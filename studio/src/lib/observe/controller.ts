import { callObservationCallback } from "./callbacks.ts";
import { captureObservationScope, sameObservationScope } from "./scope.ts";
import type { ObservationScope } from "./scope.ts";
import type {
  Observation,
  ObservationOptions,
  ObservationSource,
  ObservationState,
} from "./types.ts";

/** Current observation ownership; transport parsers and durable mutation recovery remain separate. */
export function createObservation<T>(
  options: ObservationOptions<T>,
): Observation<T> {
  const { clone, measure, classifyError, onAuthorityLost } = options;
  const limits = { ...options.limits };
  const retry = { ...options.retry };
  for (const value of [
    limits.maxRows,
    limits.maxBytes,
    retry.maxAttempts,
    retry.maxElapsedMs,
  ]) {
    if (!Number.isSafeInteger(value) || value <= 0)
      throw new TypeError("Invalid observation limits");
  }
  if (!Number.isSafeInteger(retry.delayMs) || retry.delayMs < 0)
    throw new TypeError("Invalid observation retry delay");
  let scope = captureObservationScope(options.scope);
  let source = options.source;
  let state: ObservationState<T> = {
    phase: "idle",
    scope,
    rows: [],
    stale: false,
    code: null,
  };
  let epoch = 0;
  let active: AbortController | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let visible = true;
  let started = false;
  let disposed = false;
  let attempts = 0;
  let cycleStart = 0;
  let lastClock = -Infinity;
  const listeners = new Set<(state: ObservationState<T>) => void>();
  function clock() {
    const value = retry.clock();
    if (!Number.isFinite(value) || value < lastClock)
      throw new TypeError("Invalid observation clock");
    lastClock = value;
    return value;
  }
  function snapshot(): ObservationState<T> {
    const ticket = epoch;
    const current = state;
    const rows = current.rows.map(clone);
    if (ticket !== epoch)
      throw new Error("Observation scope changed during clone");
    return Object.freeze({ ...current, rows: Object.freeze(rows) });
  }
  function reportCallback(
    code: "ObservationSubscriber" | "ObservationAuthorityCallback",
  ) {
    callObservationCallback(
      () => options.onCallbackError?.(code),
      () => {},
    );
  }
  function notify(
    listener: (state: ObservationState<T>) => void,
    value: ObservationState<T>,
  ) {
    callObservationCallback(
      () => listener(value),
      () => {
        listeners.delete(listener);
        reportCallback("ObservationSubscriber");
      },
    );
  }
  function publish(next: ObservationState<T>) {
    state = next;
    const ticket = epoch;
    for (const listener of [...listeners]) {
      if (ticket !== epoch) break;
      const copy = snapshot();
      if (ticket !== epoch) break;
      notify(listener, copy);
    }
  }
  function stop() {
    const previous = active;
    const ticket = ++epoch;
    active = null;
    if (timer !== null) clearTimeout(timer);
    timer = null;
    previous?.abort();
    return ticket;
  }
  function failure(
    ticket: number,
    kind: "transient" | "denied" | "fatal",
    code: string,
  ) {
    if (ticket !== epoch || disposed || !visible) return;
    active = null;
    if (kind === "denied") {
      publish({ phase: "denied", scope, rows: [], stale: false, code });
      if (ticket === epoch && !disposed) {
        callObservationCallback(
          () => onAuthorityLost(scope),
          () => {
            reportCallback("ObservationAuthorityCallback");
          },
        );
      }
      return;
    }
    let elapsed = 0;
    if (kind !== "fatal") {
      try {
        elapsed = clock() - cycleStart;
      } catch {
        if (ticket !== epoch || disposed || !visible) return;
        publish({
          phase: "exhausted",
          scope,
          rows: [],
          stale: false,
          code: "ObservationConfiguration",
        });
        return;
      }
    }
    if (ticket !== epoch || disposed || !visible) return;
    const exhausted =
      kind === "fatal" ||
      attempts >= retry.maxAttempts ||
      elapsed + retry.delayMs >= retry.maxElapsedMs;
    const rows = kind === "fatal" ? [] : state.rows;
    publish({
      ...state,
      rows,
      phase: exhausted ? "exhausted" : "stale",
      stale: rows.length > 0,
      code,
    });
    if (exhausted || ticket !== epoch || disposed || !visible) return;
    timer = setTimeout(() => {
      timer = null;
      if (ticket !== epoch || disposed || !visible) return;
      let elapsed: number;
      try {
        elapsed = clock() - cycleStart;
      } catch {
        if (ticket === epoch && !disposed)
          publish({
            phase: "exhausted",
            scope,
            rows: [],
            stale: false,
            code: "ObservationConfiguration",
          });
        return;
      }
      if (ticket !== epoch || disposed || !visible) return;
      if (elapsed >= retry.maxElapsedMs) {
        publish({ ...state, phase: "exhausted", code: "ObservationDeadline" });
        return;
      }
      connect();
    }, retry.delayMs);
  }
  function connect() {
    if (disposed || !visible || active || timer !== null) return;
    const controller = new AbortController();
    active = controller;
    const ticket = ++epoch;
    const currentSource = source;
    const owns = () =>
      !disposed && visible && epoch === ticket && active === controller;
    attempts++;
    publish({
      ...state,
      phase: "connecting",
      stale: state.rows.length > 0,
      code: null,
    });
    if (!owns()) return;
    void (async () => {
      try {
        for await (const incoming of currentSource(controller.signal)) {
          if (!owns()) return;
          if (!Array.isArray(incoming) || incoming.length > limits.maxRows) {
            failure(ticket, "fatal", "ObservationLimit");
            controller.abort();
            return;
          }
          const rows = incoming.map(clone);
          if (!owns()) return;
          const bytes = measure(rows);
          if (!owns()) return;
          if (
            !Number.isSafeInteger(bytes) ||
            bytes < 0 ||
            bytes > limits.maxBytes
          ) {
            failure(ticket, "fatal", "ObservationLimit");
            controller.abort();
            return;
          }
          publish({ phase: "fresh", scope, rows, stale: false, code: null });
          if (!owns()) return;
        }
        if (owns()) failure(ticket, "transient", "ObservationEnded");
      } catch (error) {
        if (!owns()) return;
        try {
          const classified = classifyError(error);
          if (!owns()) return;
          if (
            !["transient", "denied", "fatal"].includes(classified.kind) ||
            typeof classified.code !== "string"
          )
            throw new TypeError("Invalid observation classifier");
          failure(ticket, classified.kind, classified.code);
        } catch {
          if (owns()) failure(ticket, "fatal", "ObservationConfiguration");
        }
      }
    })();
  }
  function begin() {
    const ticket = stop();
    if (disposed || epoch !== ticket) return;
    attempts = 0;
    cycleStart = clock();
    if (disposed || epoch !== ticket) return;
    connect();
  }
  return {
    get state() {
      return snapshot();
    },
    start() {
      if (disposed || started) return;
      started = true;
      begin();
    },
    setVisible(next) {
      if (disposed || visible === next) return;
      visible = next;
      if (!next) {
        const ticket = stop();
        if (disposed || epoch !== ticket) return;
        publish({
          ...state,
          phase: state.rows.length ? "stale" : "idle",
          stale: state.rows.length > 0,
        });
      } else if (started) begin();
    },
    rebind(next: ObservationScope, nextSource: ObservationSource<T>) {
      if (disposed) return;
      const captured = captureObservationScope(next);
      const same = sameObservationScope(scope, captured);
      const ticket = stop();
      if (disposed || epoch !== ticket) return;
      scope = captured;
      source = nextSource;
      publish({
        phase: "idle",
        scope,
        rows: same ? state.rows : [],
        stale: same && state.rows.length > 0,
        code: null,
      });
      if (started && !disposed && epoch === ticket) begin();
    },
    retry() {
      if (!disposed && started && !active && timer === null) begin();
    },
    subscribe(listener) {
      if (disposed) {
        notify(listener, snapshot());
        return () => {};
      }
      listeners.add(listener);
      notify(listener, snapshot());
      return () => {
        listeners.delete(listener);
      };
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      stop();
      publish({ phase: "disposed", scope, rows: [], stale: false, code: null });
      listeners.clear();
    },
  };
}
