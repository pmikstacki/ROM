import { currentTime } from "./clock.ts";
import { untilAborted } from "./cancellation.ts";
import { identity, samePrincipal } from "./identity.ts";
import type {
  SessionCheck,
  SessionIdentity,
  SessionLifecycle,
  SessionLifecycleOptions,
  SessionLifecycleState,
  SessionTransition,
} from "./types.ts";

/** A controller owns one acquisition; its ticket guards every external callback. */
export function createSessionLifecycle(
  input: SessionLifecycleOptions,
): SessionLifecycle {
  const options = { ...input };
  let state: SessionLifecycleState = {
    status: "anonymous",
    identity: null,
    transientCode: null,
  };
  let epoch = 0,
    disposed = false;
  let flight: Promise<SessionCheck> | null = null;
  let active: AbortController | null = null;
  let cancelExpiry: (() => void) | null = null;
  const listeners = new Set<(state: SessionLifecycleState) => void>();
  const snapshot = () => structuredClone(state);
  const current = (ticket: number) => !disposed && ticket === epoch;
  function publish(ticket: number) {
    for (const listener of listeners) {
      if (!current(ticket)) break;
      try {
        listener(snapshot());
      } catch {
        /* One host listener cannot prevent invalidation. */
      }
    }
  }
  function invalidate() {
    const ticket = ++epoch;
    const old = active;
    active = null;
    flight = null;
    cancelExpiry?.();
    cancelExpiry = null;
    old?.abort();
    return ticket;
  }
  function clearCredentials() {
    try {
      options.driver.invalidate?.();
    } catch {
      /* Private authority remains cleared even if the host fails. */
    }
  }
  async function hook(
    transition: SessionTransition,
    ticket: number,
  ): Promise<boolean> {
    if (!current(ticket)) return false;
    try {
      await options.onTransition?.(structuredClone(transition));
    } catch {
      if (current(ticket)) {
        state = {
          status: "transient",
          identity: null,
          transientCode: "transition_failed",
        };
        clearCredentials();
        publish(ticket);
      }
      return false;
    }
    return current(ticket);
  }
  function expire() {
    if (
      disposed ||
      !state.identity ||
      state.identity.expiresAt > currentTime(options.now)
    )
      return;
    const previous = state.identity;
    state = { status: "anonymous", identity: null, transientCode: null };
    const ticket = invalidate();
    if (!current(ticket)) return;
    clearCredentials();
    void hook({ kind: "cleared", previous, next: null }, ticket).then(
      (valid) => {
        if (valid) publish(ticket);
      },
    );
  }
  function armExpiry(value: SessionIdentity) {
    cancelExpiry?.();
    const schedule =
      options.schedule ??
      ((task: () => void, ms: number) => {
        const timer = setTimeout(task, ms);
        return () => clearTimeout(timer);
      });
    const delay = Math.min(
      2147483647,
      Math.max(0, (value.expiresAt - currentTime(options.now)) * 1000),
    );
    const expected = value;
    cancelExpiry = schedule(() => {
      cancelExpiry = null;
      if (disposed || state.identity !== expected) return;
      expire();
      if (state.identity === expected) armExpiry(expected);
    }, delay);
  }
  async function apply(
    result: SessionCheck,
    ticket: number,
  ): Promise<SessionCheck> {
    if (!current(ticket)) return { status: "anonymous" };
    expire();
    if (!current(ticket)) return { status: "anonymous" };
    if (
      !result ||
      !["authenticated", "anonymous", "denied", "transient"].includes(
        result.status,
      )
    )
      return apply({ status: "transient", code: "invalid_response" }, ticket);
    if (result.status === "authenticated") {
      let next: SessionIdentity;
      try {
        next = identity(result.identity);
      } catch {
        return apply({ status: "transient", code: "invalid_response" }, ticket);
      }
      if (next.expiresAt <= currentTime(options.now))
        return apply({ status: "anonymous" }, ticket);
      const previous = state.identity;
      state = { status: "checking", identity: null, transientCode: null };
      cancelExpiry?.();
      cancelExpiry = null;
      if (
        !(await hook(
          {
            kind:
              previous && samePrincipal(previous, next) ? "renewed" : "changed",
            previous,
            next,
          },
          ticket,
        ))
      )
        return { status: "anonymous" };
      if (next.expiresAt <= currentTime(options.now))
        return apply({ status: "anonymous" }, ticket);
      state = { status: "authenticated", identity: next, transientCode: null };
      armExpiry(next);
      publish(ticket);
      return current(ticket)
        ? { status: "authenticated", identity: identity(next) }
        : { status: "anonymous" };
    }
    if (result.status === "transient") {
      const previous = state.identity;
      const code = /^[a-z][a-z0-9_]{0,63}$/.test(result.code)
        ? result.code
        : "unavailable";
      state = { status: "transient", identity: previous, transientCode: code };
      if (
        !(await hook({ kind: "transient", previous, next: previous }, ticket))
      )
        return { status: "anonymous" };
      expire();
      if (!current(ticket)) return { status: "anonymous" };
      publish(ticket);
      return current(ticket)
        ? { status: "transient", code }
        : { status: "anonymous" };
    }
    const previous = state.identity;
    state = { status: result.status, identity: null, transientCode: null };
    cancelExpiry?.();
    cancelExpiry = null;
    flight = null;
    clearCredentials();
    if (!(await hook({ kind: "cleared", previous, next: null }, ticket)))
      return { status: "anonymous" };
    publish(ticket);
    return current(ticket)
      ? { status: result.status }
      : { status: "anonymous" };
  }
  function refresh(): Promise<SessionCheck> {
    expire();
    if (disposed) return Promise.resolve({ status: "anonymous" });
    if (flight) return flight;
    const ticket = ++epoch,
      controller = new AbortController();
    active = controller;
    state = { ...state, status: "checking" };
    // Defer driver invocation so synchronous listeners cannot outrun the flight slot.
    const pending = Promise.resolve()
      .then(async () => {
        if (!current(ticket)) return { status: "anonymous" } as SessionCheck;
        let result: SessionCheck;
        try {
          result = await untilAborted(
            options.driver.check(controller.signal),
            controller.signal,
          );
        } catch {
          result = { status: "transient", code: "unavailable" };
        }
        if (!current(ticket)) return { status: "anonymous" } as SessionCheck;
        return apply(result, ticket);
      })
      .finally(() => {
        if (flight === pending) flight = null;
        if (active === controller) active = null;
      });
    flight = pending;
    publish(ticket);
    return pending;
  }
  async function logout() {
    if (disposed) return;
    const previous = state.identity;
    state = { status: "anonymous", identity: null, transientCode: null };
    const ticket = invalidate();
    if (!current(ticket)) return;
    const controller = new AbortController();
    active = controller;
    // Start logout before host hooks can acquire replacement credentials.
    let pending: Promise<void>;
    try {
      pending = options.driver.logout(controller.signal);
    } catch {
      pending = Promise.reject(Error("logout_unconfirmed"));
    }
    void pending.catch(() => {});
    // The driver captured its logout credential synchronously; clear local authority before hooks.
    if (current(ticket)) clearCredentials();
    const valid = await hook({ kind: "cleared", previous, next: null }, ticket);
    if (valid) publish(ticket);
    try {
      await untilAborted(pending, controller.signal);
    } catch {
      if (current(ticket)) throw Error("logout_unconfirmed");
    } finally {
      if (active === controller) active = null;
    }
  }
  return {
    get state() {
      expire();
      return snapshot();
    },
    refresh,
    logout,
    dismissTransient() {
      expire();
      if (disposed || state.status !== "transient") return;
      state = {
        status: state.identity ? "authenticated" : "anonymous",
        identity: state.identity,
        transientCode: null,
      };
      publish(epoch);
    },
    subscribe(listener) {
      expire();
      listeners.add(listener);
      try {
        listener(snapshot());
      } catch {
        /* Subscription remains removable. */
      }
      return () => {
        listeners.delete(listener);
      };
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      state = { status: "disposed", identity: null, transientCode: null };
      listeners.clear();
      invalidate();
      clearCredentials();
    },
  };
}
