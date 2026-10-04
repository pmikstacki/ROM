import { parseWire, stringifyWire } from "./codec.ts";
import { boundedBytes, discardBody, jsonResponse } from "./request.ts";
import { projected, record, text, unsigned } from "./validation.ts";
import type {
  BlobCapabilities,
  BlobReservation,
  ClientOptions,
  ProjectedView,
  WireValue,
} from "./types.ts";

const operations = ["reserve", "upload", "download", "detach"] as const;
type Run = <T>(
  execute: (signal: AbortSignal) => Promise<T>,
  external?: AbortSignal,
) => Promise<T>;

function capabilities(value: WireValue): BlobCapabilities {
  const object = record(value),
    limits = record(object.limits);
  if (unsigned(object.version, object, "version") !== 1n)
    throw Error("blob capabilities version");
  if (!Array.isArray(object.stores) || object.stores.length > 32)
    throw Error("blob stores limit");
  const stores = object.stores.map(text);
  if (new Set(stores).size !== stores.length)
    throw Error("duplicate blob stores");
  const supported = object.operations;
  if (
    !Array.isArray(supported) ||
    supported.length !== operations.length ||
    operations.some((op) => !supported.includes(op))
  )
    throw Error("blob operations");
  const positive = (name: string) => {
    const integer = unsigned(limits[name], limits, name);
    if (integer < 1n || integer > BigInt(Number.MAX_SAFE_INTEGER))
      throw Error("blob limit integer");
    return Number(integer);
  };
  return {
    version: 1,
    resource_kind: text(object.resource_kind),
    stores,
    limits: {
      blob_bytes: positive("blob_bytes"),
      chunk_bytes: positive("chunk_bytes"),
      chunks: positive("chunks"),
    },
    operations: [...operations],
  };
}

/** Optional host binary transport. The Resource projection remains the result authority. */
export function blobClient(
  options: ClientOptions,
  run: Run,
  generation: () => number,
) {
  let capabilityEpoch = 0;
  let admitted: { generation: number; value: BlobCapabilities } | undefined;
  const base = () => {
    const api = options.base.replace(/\/$/, "");
    if (!api.endsWith("/api"))
      throw Error("blob transport requires an API base");
    return `${api.slice(0, -4)}/blobs`;
  };
  const current = () => {
    if (!admitted || admitted.generation !== generation())
      throw Error("blob capabilities must be admitted");
    return admitted.value;
  };
  const response = (
    value: WireValue,
    status: string,
    kind: string,
    id: string,
  ): ProjectedView => {
    const result = record(value);
    if (result.status !== status) throw Error("blob response status");
    const view = projected(result.resource, kind, id);
    if (view.value === null) throw Error("missing blob projection");
    return view;
  };
  async function request(
    path: string,
    method: "GET" | "POST",
    signal: AbortSignal,
    body?: string | Blob,
  ) {
    const csrf = options.csrf?.();
    return (options.fetch ?? globalThis.fetch)(`${base()}/${path}`, {
      method,
      credentials: "same-origin",
      redirect: "error",
      signal,
      headers: {
        accept: "application/json",
        ...(method === "POST"
          ? {
              "content-type":
                body instanceof Blob
                  ? "application/octet-stream"
                  : "application/json",
              ...(csrf ? { "x-rom-csrf": csrf } : {}),
            }
          : {}),
      },
      ...(body === undefined ? {} : { body }),
    });
  }
  return {
    async blobCapabilities(signal?: AbortSignal) {
      const started = generation();
      const epoch = ++capabilityEpoch;
      admitted = undefined;
      const value = capabilities(
        await run(
          async (s) =>
            jsonResponse(await request("capabilities", "GET", s), options, s),
          signal,
        ),
      );
      if (started !== generation()) throw Error("session changed");
      if (epoch !== capabilityEpoch)
        throw Error("superseded blob capabilities");
      admitted = { generation: started, value: structuredClone(value) };
      return value;
    },
    async reserveBlob(input: BlobReservation, signal?: AbortSignal) {
      const cap = current();
      const frozen = record(
        parseWire(stringifyWire(input as unknown as WireValue)),
      );
      const id = text(frozen.id),
        store = text(frozen.store),
        digest = text(frozen.digest);
      text(frozen.idempotency);
      const bytes = unsigned(frozen.bytes, frozen, "bytes");
      if (!cap.stores.includes(store)) throw Error("unavailable blob store");
      if (!/^[0-9a-f]{64}$/.test(digest)) throw Error("invalid blob digest");
      if (bytes > BigInt(cap.limits.blob_bytes))
        throw Error("blob bytes limit");
      const body = stringifyWire(frozen);
      return response(
        await run(
          async (s) =>
            jsonResponse(await request("reserve", "POST", s, body), options, s),
          signal,
        ),
        "reserved",
        cap.resource_kind,
        id,
      );
    },
    async uploadBlob(id: string, file: Blob, signal?: AbortSignal) {
      const cap = current();
      text(id);
      if (
        !(file instanceof Blob) ||
        file.size > cap.limits.blob_bytes ||
        BigInt(file.size) >
          BigInt(cap.limits.chunk_bytes) * BigInt(cap.limits.chunks)
      )
        throw Error("blob upload limit");
      const frozen = file.slice(0, file.size, "application/octet-stream");
      return response(
        await run(
          async (s) =>
            jsonResponse(
              await request(
                `upload?id=${encodeURIComponent(id)}`,
                "POST",
                s,
                frozen,
              ),
              options,
              s,
            ),
          signal,
        ),
        "attached",
        cap.resource_kind,
        id,
      );
    },
    async downloadBlob(id: string, signal?: AbortSignal) {
      const cap = current();
      text(id);
      return run(async (s) => {
        const result = await request(
          `attachment?id=${encodeURIComponent(id)}`,
          "GET",
          s,
        );
        if (!result.ok) {
          await jsonResponse(result, options, s);
          throw Error("blob download failed");
        }
        if (
          result.headers.get("content-type")?.split(";")[0].trim() !==
          "application/octet-stream"
        ) {
          discardBody(result);
          throw Error("invalid binary content type");
        }
        return boundedBytes(
          result,
          Math.min(cap.limits.blob_bytes, options.maxBytes ?? 1048576),
          s,
        );
      }, signal);
    },
    async detachBlob(id: string, signal?: AbortSignal) {
      const cap = current();
      text(id);
      const body = stringifyWire({ id });
      return response(
        await run(
          async (s) =>
            jsonResponse(await request("detach", "POST", s, body), options, s),
          signal,
        ),
        "detached",
        cap.resource_kind,
        id,
      );
    },
  };
}
