import { parseWire, stringifyWire } from "./codec.ts";
import type { ClientOptions, WireValue } from "./types.ts";
import { record, text } from "./validation.ts";
export class RemoteError extends Error {
  readonly category: string;
  readonly status: number;
  constructor(category: string, status: number) {
    super(category);
    this.name = "RemoteError";
    this.category = category;
    this.status = status;
  }
}
export function discardBody(response: Response): void {
  void response.body?.cancel().catch(() => {});
}
export async function boundedBytes(
  response: Response,
  maxBytes: number,
  signal?: AbortSignal,
): Promise<Uint8Array> {
  const declared = response.headers.get("content-length");
  if (
    declared !== null &&
    (!/^\d+$/.test(declared) || BigInt(declared) > BigInt(maxBytes))
  ) {
    discardBody(response);
    throw new Error("response bytes limit");
  }
  if (!response.body) return new Uint8Array();
  const reader = response.body.getReader();
  const cancel = () => {
    void reader.cancel().catch(() => {});
  };
  signal?.addEventListener("abort", cancel, { once: true });
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    if (signal?.aborted) throw signal.reason ?? Error("request aborted");
    for (;;) {
      const { value, done } = await reader.read();
      if (signal?.aborted) throw signal.reason ?? Error("request aborted");
      if (done) break;
      size += value.byteLength;
      if (size > maxBytes) throw new Error("response bytes limit");
      chunks.push(value);
    }
    const bytes = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) {
      bytes.set(chunk, offset);
      offset += chunk.byteLength;
    }
    return bytes;
  } finally {
    signal?.removeEventListener("abort", cancel);
    cancel();
    reader.releaseLock();
  }
}
export async function boundedBody(
  response: Response,
  maxBytes: number,
  signal?: AbortSignal,
): Promise<string> {
  return new TextDecoder("utf-8", { fatal: true }).decode(
    await boundedBytes(response, maxBytes, signal),
  );
}
export async function jsonResponse(
  response: Response,
  options: ClientOptions,
  signal: AbortSignal,
): Promise<WireValue> {
  if (
    response.headers.get("content-type")?.split(";")[0].trim() !==
    "application/json"
  ) {
    discardBody(response);
    throw new Error("invalid response content type");
  }
  const value = parseWire(
    await boundedBody(response, options.maxBytes ?? 1048576, signal),
    options.maxBytes ?? 1048576,
  );
  if (!response.ok)
    throw new RemoteError(text(record(value).error), response.status);
  return value;
}
export async function post(
  options: ClientOptions,
  route: string,
  body: WireValue,
  signal: AbortSignal,
): Promise<WireValue> {
  const csrf = options.csrf?.();
  const response = await (options.fetch ?? globalThis.fetch)(
    `${options.base.replace(/\/$/, "")}/${route}`,
    {
      method: "POST",
      credentials: "same-origin",
      redirect: "error",
      signal,
      headers: {
        "content-type": "application/json",
        accept: "application/json",
        ...(csrf ? { "x-rom-csrf": csrf } : {}),
      },
      body: stringifyWire(body),
    },
  );
  return jsonResponse(response, options, signal);
}
