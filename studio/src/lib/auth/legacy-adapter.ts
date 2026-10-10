import { createBrowserSessionTransport } from "./browser-transport.ts";
import type { BrowserSession } from "./protocol-types.ts";
export class SessionExpiredError extends Error {
  constructor() {
    super("Session expired.");
    this.name = "SessionExpiredError";
  }
}
/** Confirmed authority loss; callers must clear private views without calling it expiry. */
export class SessionDeniedError extends Error {
  constructor() {
    super("Session denied.");
    this.name = "SessionDeniedError";
  }
}
/** Compatibility transport API; it does not invent a durable principal authority. */
export function createBrowserAuth(
  base: string,
  fetcher: typeof fetch = globalThis.fetch,
) {
  const transport = createBrowserSessionTransport({
    base,
    fetch: fetcher,
    now: () => Date.now() / 1000,
  });
  let epoch = 0;
  return {
    async refresh(): Promise<BrowserSession> {
      const ticket = epoch;
      const result = await transport.refresh(new AbortController().signal);
      if (ticket !== epoch) throw Error("Session changed.");
      switch (result.status) {
        case "session":
          return structuredClone(result.session);
        case "expired":
          throw new SessionExpiredError();
        case "changed":
          throw Error("Session changed.");
        case "transient":
          throw Error(
            result.code === "invalid_response"
              ? "Invalid session response."
              : "Session unavailable.",
          );
        case "denied":
          throw new SessionDeniedError();
      }
    },
    providers: () => transport.providers(new AbortController().signal),
    async logout() {
      ++epoch;
      try {
        await transport.logout(new AbortController().signal);
      } catch {
        throw Error("Logout was not confirmed.");
      }
    },
    csrf: transport.csrf,
    loginUrl: (id: string) => `${base}auth/login/${encodeURIComponent(id)}`,
  };
}
