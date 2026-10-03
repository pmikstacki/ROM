import { parseWire, stringifyWire } from "./codec.ts";
import { projectedRows } from "./validation.ts";
import { boundedBody, RemoteError } from "./request.ts";
import type {
  ClientOptions,
  ProjectedView,
  QuerySpec,
  WireValue,
} from "./types.ts";
/** A live query is one bounded, authenticated POST stream. Reconnection requires a fresh snapshot. */
export async function* live(
  options: ClientOptions,
  kind: string,
  query: QuerySpec,
  signal: AbortSignal,
  current: () => boolean,
): AsyncIterable<ProjectedView[]> {
  const csrf = options.csrf?.();
  const response = await (options.fetch ?? globalThis.fetch)(
    `${options.base.replace(/\/$/, "")}/live`,
    {
      method: "POST",
      credentials: "same-origin",
      redirect: "error",
      signal,
      headers: {
        "content-type": "application/json",
        accept: "text/event-stream",
        ...(csrf ? { "x-rom-csrf": csrf } : {}),
      },
      body: stringifyWire({ kind, query } as unknown as WireValue),
    },
  );
  if (!current()) throw new Error("session changed");
  if (!response.ok) {
    await boundedBody(response, options.maxBytes ?? 1048576);
    throw new RemoteError("stream rejected", response.status);
  }
  if (
    response.headers.get("content-type")?.split(";")[0].trim() !==
      "text/event-stream" ||
    !response.body
  )
    throw new Error("invalid stream content type");
  const reader = response.body.getReader(),
    decoder = new TextDecoder("utf-8", { fatal: true });
  const maxBytes = options.maxBytes ?? 1048576;
  let pending = "",
    frameBytes = 0,
    event = "",
    data: string[] = [];
  const frame = (): ProjectedView[] | undefined => {
    const payload = data.join("\n"),
      type = event;
    event = "";
    data = [];
    frameBytes = 0;
    if (payload === "") return undefined;
    if (type === "error") throw new RemoteError(payload, 0);
    if (type !== "data") throw new Error("unsupported stream event");
    return projectedRows(
      parseWire(payload, maxBytes),
      kind,
      options.maxRows ?? 10000,
    );
  };
  try {
    for (;;) {
      if (signal.aborted) throw signal.reason ?? new Error("stream aborted");
      const { value, done } = await reader.read();
      if (!current()) throw new Error("session changed");
      if (done) {
        if (pending !== "" || data.length > 0)
          throw new Error("truncated stream frame");
        return;
      }
      pending += decoder.decode(value, { stream: true });
      for (;;) {
        const boundary = pending.indexOf("\n");
        if (boundary < 0) break;
        const raw = pending.slice(0, boundary),
          line = raw.endsWith("\r") ? raw.slice(0, -1) : raw;
        pending = pending.slice(boundary + 1);
        frameBytes += new TextEncoder().encode(raw).length + 1;
        if (frameBytes > maxBytes) throw new Error("stream frame limit");
        if (line === "") {
          const rows = frame();
          if (rows !== undefined) yield rows;
          continue;
        }
        if (line.startsWith(":")) continue;
        const colon = line.indexOf(":"),
          field = colon < 0 ? line : line.slice(0, colon);
        let content = colon < 0 ? "" : line.slice(colon + 1);
        if (content.startsWith(" ")) content = content.slice(1);
        if (field === "event") event = content;
        else if (field === "data") data.push(content);
      }
      if (frameBytes + new TextEncoder().encode(pending).length > maxBytes)
        throw new Error("stream frame limit");
    }
  } finally {
    await reader.cancel().catch(() => {});
    reader.releaseLock();
  }
}
