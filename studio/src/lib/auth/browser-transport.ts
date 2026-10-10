import { providerResponse } from "./provider-response.ts";
import { currentTime } from "./clock.ts";
import { InvalidSessionResponse, sessionResponse } from "./browser-response.ts";
import { sessionRequest } from "./browser-request.ts";
import type {
  BrowserProtocolResult,
  BrowserSessionTransport,
  BrowserTransportOptions,
} from "./protocol-types.ts";
interface Flight {
  epoch: number;
  controller: AbortController;
  pending: Promise<BrowserProtocolResult>;
  waiters: Set<() => void>;
}
/** Shared acquisition has per-waiter cancellation and one deadline-bound driver. */
export function createBrowserSessionTransport(
  input: BrowserTransportOptions,
): BrowserSessionTransport {
  const options = { ...input };
  let token: string | undefined,
    expiry = 0,
    epoch = 0;
  let flight: Flight | null = null;
  const clear = () => {
    token = undefined;
    expiry = 0;
  };
  function invalidate() {
    ++epoch;
    clear();
    const old = flight;
    flight = null;
    old?.controller.abort();
    if (old) for (const stop of [...old.waiters]) stop();
  }
  function acquire(): Flight {
    const controller = new AbortController(),
      ticket = epoch;
    let complete!: (result: BrowserProtocolResult) => void;
    const pending = new Promise<BrowserProtocolResult>((resolve) => {
      complete = resolve;
    });
    const entry: Flight = {
      epoch: ticket,
      controller,
      pending,
      waiters: new Set(),
    };
    flight = entry;
    void (async (): Promise<BrowserProtocolResult> => {
      if (controller.signal.aborted || ticket !== epoch)
        return { status: "changed" };
      try {
        const response = await sessionRequest(
          options,
          controller.signal,
          "session",
        );
        if (ticket !== epoch || controller.signal.aborted)
          return { status: "changed" };
        const result = sessionResponse(
          response.body,
          response.status,
          currentTime(options.now),
        );
        if (ticket !== epoch || controller.signal.aborted)
          return { status: "changed" };
        if (result.status === "session" && result.session.authenticated) {
          token = result.session.csrf_token;
          expiry = result.session.expires_at;
        } else if (result.status !== "transient") clear();
        return result;
      } catch (error) {
        if (ticket !== epoch || controller.signal.aborted)
          return { status: "changed" };
        return {
          status: "transient",
          code:
            error instanceof InvalidSessionResponse
              ? "invalid_response"
              : "unavailable",
        };
      }
    })().then((result) => {
      if (flight === entry) flight = null;
      complete(result);
    });
    return entry;
  }
  function refresh(signal: AbortSignal): Promise<BrowserProtocolResult> {
    if (signal.aborted) return Promise.resolve({ status: "changed" });
    const entry = flight ?? acquire();
    return new Promise((resolve) => {
      let settled = false;
      const finish = (result: BrowserProtocolResult) => {
        if (settled) return;
        settled = true;
        signal.removeEventListener("abort", stop);
        entry.waiters.delete(stop);
        if (entry.waiters.size === 0 && flight === entry) {
          flight = null;
          entry.controller.abort();
        }
        resolve(structuredClone(result));
      };
      const stop = () => finish({ status: "changed" });
      entry.waiters.add(stop);
      signal.addEventListener("abort", stop, { once: true });
      if (signal.aborted) stop();
      void entry.pending.then((result) =>
        finish(entry.epoch === epoch ? result : { status: "changed" }),
      );
    });
  }
  return {
    refresh,
    async providers(signal) {
      const ticket = epoch;
      try {
        const response = await sessionRequest(options, signal, "providers");
        if (ticket !== epoch || signal.aborted) throw Error("Session changed.");
        if (response.status < 200 || response.status >= 300)
          throw Error("Invalid provider response.");
        return providerResponse(response.body);
      } catch {
        throw Error(
          ticket !== epoch || signal.aborted
            ? "Session changed."
            : "Invalid provider response.",
        );
      }
    },
    async logout(signal) {
      const credential = expiry > currentTime(options.now) ? token : undefined;
      invalidate();
      if (!credential) return;
      const response = await sessionRequest(
        options,
        signal,
        "logout",
        credential,
      );
      if (response.status < 200 || response.status >= 300)
        throw Error("logout_unconfirmed");
    },
    invalidate,
    csrf() {
      if (expiry <= currentTime(options.now)) clear();
      return token;
    },
  };
}
