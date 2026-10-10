import { parseWire } from "../client/codec.ts";
import { boundedBody, discardBody } from "../client/request.ts";
import { deadline } from "../client/deadline.ts";
import { InvalidSessionResponse } from "./browser-response.ts";
import type { BrowserTransportOptions } from "./protocol-types.ts";
export async function sessionRequest(
  options: BrowserTransportOptions,
  signal: AbortSignal,
  route: "session" | "providers" | "logout",
  token?: string,
): Promise<{ status: number; body: unknown }> {
  const controller = new AbortController();
  const abort = () => controller.abort();
  if (signal.aborted) abort();
  else signal.addEventListener("abort", abort, { once: true });
  const pending = (async () => {
    const response = await (options.fetch ?? globalThis.fetch)(
      `${options.base}auth/${route}`,
      {
        method: route === "logout" ? "POST" : "GET",
        credentials: "same-origin",
        redirect: "error",
        signal: controller.signal,
        headers:
          route === "logout"
            ? { "x-rom-csrf": token! }
            : { accept: "application/json" },
      },
    );
    if (controller.signal.aborted) {
      discardBody(response);
      throw Error("session cancelled");
    }
    if (route === "logout") {
      discardBody(response);
      return { status: response.status, body: null };
    }
    if (
      response.headers.get("content-type")?.split(";")[0].trim() !==
      "application/json"
    ) {
      discardBody(response);
      throw new InvalidSessionResponse();
    }
    let body: unknown;
    try {
      body = parseWire(
        await boundedBody(response, 65536, controller.signal),
        65536,
      );
    } catch {
      throw new InvalidSessionResponse();
    }
    return { status: response.status, body };
  })();
  try {
    return await deadline(pending, signal, options.timeoutMs ?? 10000, abort);
  } finally {
    signal.removeEventListener("abort", abort);
    abort();
  }
}
