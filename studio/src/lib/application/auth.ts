import { parseWire } from "../client/codec.ts";
import { boundedBody } from "../client/request.ts";
export class SessionExpiredError extends Error {
  constructor() {
    super("Session expired.");
    this.name = "SessionExpiredError";
  }
}
export interface BrowserSession {
  authenticated: boolean;
  generation: string;
  csrf_token?: string;
  user_id?: string;
  expires_at?: number;
}
export interface ProviderChoice {
  id: string;
  label: string;
}
function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw Error("Invalid session response.");
  return value as Record<string, unknown>;
}
function text(value: unknown): string {
  if (typeof value !== "string" || !value || value.length > 4096)
    throw Error("Invalid session response.");
  return value;
}
export function createBrowserAuth(
  base: string,
  fetcher: typeof fetch = globalThis.fetch,
) {
  let session: BrowserSession | undefined;
  let epoch = 0;
  async function get(route: string): Promise<unknown> {
    const controller = new AbortController(),
      timer = setTimeout(() => controller.abort(), 10000);
    try {
      const response = await fetcher(`${base}auth/${route}`, {
        credentials: "same-origin",
        redirect: "error",
        signal: controller.signal,
        headers: { accept: "application/json" },
      });
      if (
        !response.ok ||
        response.headers.get("content-type")?.split(";")[0] !==
          "application/json"
      )
        throw Error("Session unavailable.");
      return parseWire(await boundedBody(response, 65536));
    } finally {
      clearTimeout(timer);
    }
  }
  async function refresh() {
    const started = epoch;
    const response = await get("session");
    if (started !== epoch) throw Error("Session changed.");
    const value = record(response);
    if (typeof value.authenticated !== "boolean")
      throw Error("Invalid session response.");
    const next: BrowserSession = {
      authenticated: value.authenticated,
      generation: text(value.generation),
    };
    if (next.authenticated) {
      next.csrf_token = text(value.csrf_token);
      next.user_id = text(value.user_id);
      if (
        typeof value.expires_at !== "bigint" &&
        typeof value.expires_at !== "number"
      )
        throw Error("Invalid session expiry.");
      next.expires_at = Number(value.expires_at);
      if (
        !Number.isSafeInteger(next.expires_at) ||
        next.expires_at <= Date.now() / 1000
      ) {
        session = undefined;
        throw new SessionExpiredError();
      }
    } else if (value.csrf_token !== undefined || value.user_id !== undefined)
      throw Error("Unauthenticated session exposed authority.");
    if (started !== epoch) throw Error("Session changed.");
    session = next;
    return next;
  }
  async function providers() {
    const value = record(await get("providers"));
    if (!Array.isArray(value.providers) || value.providers.length > 100)
      throw Error("Invalid provider response.");
    const choices = value.providers.map((item) => {
      const entry = record(item);
      return { id: text(entry.id), label: text(entry.label) };
    });
    if (new Set(choices.map((p) => p.id)).size !== choices.length)
      throw Error("Duplicate provider ID.");
    const primary = value.primary === null ? null : text(value.primary);
    if (primary !== null && !choices.some((p) => p.id === primary))
      throw Error("Invalid primary provider.");
    return { providers: choices, primary };
  }
  async function logout() {
    epoch++;
    const token = session?.csrf_token;
    session = undefined;
    if (!token) return;
    const controller = new AbortController(),
      timer = setTimeout(() => controller.abort(), 10000);
    try {
      const response = await fetcher(`${base}auth/logout`, {
        method: "POST",
        credentials: "same-origin",
        redirect: "error",
        signal: controller.signal,
        headers: { "x-rom-csrf": token },
      });
      if (!response.ok) throw Error("Logout was not confirmed.");
    } finally {
      clearTimeout(timer);
    }
  }
  return {
    refresh,
    providers,
    logout,
    csrf: () => session?.csrf_token,
    loginUrl: (id: string) => `${base}auth/login/${encodeURIComponent(id)}`,
  };
}
