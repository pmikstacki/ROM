import { pageLimit } from "./query.ts";
import { parseWire, stringifyWire } from "./codec.ts";
import { projectedRows, record, text } from "./validation.ts";
import { boundedBody, RemoteError } from "./request.ts";
import { deadline } from "./deadline.ts";
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
  const opening = new AbortController();
  const forwardAbort = () => opening.abort(signal.reason);
  if (signal.aborted) forwardAbort();
  else signal.addEventListener("abort", forwardAbort, { once: true });
  let response: Response;
  try {
    response = await deadline(
      (options.fetch ?? globalThis.fetch)(
        `${options.base.replace(/\/$/, "")}/live`,
        {
          method: "POST",
          credentials: "same-origin",
          redirect: "error",
          signal: opening.signal,
          headers: {
            "content-type": "application/json",
            accept: "text/event-stream",
            ...(csrf ? { "x-rom-csrf": csrf } : {}),
          },
          body: stringifyWire({ kind, query } as unknown as WireValue),
        },
      ),
      signal,
      options.timeoutMs ?? 15000,
      () => opening.abort(Error("stream timeout")),
    );
  } catch (error) {
    opening.abort(error);
    throw error;
  } finally {
    signal.removeEventListener("abort", forwardAbort);
  }
  if (!current()) {
    opening.abort();
    throw new Error("session changed");
  }
  if (!response.ok) {
    const body = await deadline(
      boundedBody(response, options.maxBytes ?? 1048576),
      signal,
      options.timeoutMs ?? 15000,
      () => opening.abort(),
    );
    if (
      response.headers.get("content-type")?.split(";")[0].trim() !==
      "application/json"
    )
      throw new Error("invalid response content type");
    throw new RemoteError(
      text(record(parseWire(body, options.maxBytes ?? 1048576)).error),
      response.status,
    );
  }
  if (
    response.headers.get("content-type")?.split(";")[0].trim() !==
      "text/event-stream" ||
    !response.body
  ) {
    opening.abort();
    void response.body?.cancel().catch(() => {});
    throw new Error("invalid stream content type");
  }
  const reader = response.body.getReader(),
    decoder = new TextDecoder("utf-8", { fatal: true });
  const cancel = () => {
    opening.abort();
    void reader.cancel().catch(() => {});
  };
  signal.addEventListener("abort", cancel, { once: true });
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
      pageLimit(query, options.maxRows ?? 10000),
    );
  };
  try {
    for (;;) {
      if (signal.aborted) throw signal.reason ?? new Error("stream aborted");
      const { value, done } = await deadline(
        reader.read(),
        signal,
        options.timeoutMs ?? 15000,
        cancel,
      );
      if (signal.aborted) throw signal.reason ?? Error("stream aborted");
      if (!current()) throw new Error("session changed");
      if (done) {
        pending += decoder.decode();
        if (pending !== "" || data.length > 0)
          throw new Error("truncated stream frame");
        throw new Error("live stream closed");
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
    signal.removeEventListener("abort", cancel);
    opening.abort();
    void reader.cancel().catch(() => {});
    reader.releaseLock();
  }
}
