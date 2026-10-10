import { createBrowserSessionTransport } from "./browser-transport.ts";
import type {
  BrowserSessionDriver,
  BrowserSessionDriverOptions,
} from "./types.ts";

/** ROM Studio's human-session profile; host authority is explicit. */
export function createBrowserSessionDriver(
  input: BrowserSessionDriverOptions,
): BrowserSessionDriver {
  const options = { ...input };
  if (
    !options.authority ||
    options.authority.length > 4096 ||
    !options.base ||
    (options.timeoutMs !== undefined &&
      (!Number.isSafeInteger(options.timeoutMs) ||
        options.timeoutMs <= 0 ||
        options.timeoutMs > 2147483647))
  )
    throw Error("invalid browser session configuration");
  const transport = createBrowserSessionTransport({
    ...options,
    base: `${options.base.replace(/\/$/, "")}/`,
  });
  let epoch = 0;
  return {
    async check(signal) {
      const ticket = epoch;
      const result = await transport.refresh(signal);
      if (ticket !== epoch || signal.aborted) return { status: "anonymous" };
      if (result.status === "transient" || result.status === "denied")
        return result;
      if (result.status !== "session" || !result.session.authenticated)
        return { status: "anonymous" };
      return {
        status: "authenticated",
        identity: {
          principal: {
            authority: options.authority,
            kind: "human",
            subject: result.session.user_id,
          },
          generation: result.session.generation,
          expiresAt: result.session.expires_at,
        },
      };
    },
    logout(signal) {
      ++epoch;
      return transport.logout(signal);
    },
    invalidate() {
      ++epoch;
      transport.invalidate();
    },
    csrf: transport.csrf,
  };
}
