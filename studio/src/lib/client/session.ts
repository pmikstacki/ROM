import { parseWire, stringifyWire } from "./codec.ts";
import { deadline } from "./deadline.ts";
import { blobClient } from "./blobs.ts";
import { mutationResult } from "./mutation.ts";
import { pageLimit } from "./query.ts";
import { post, RemoteError } from "./request.ts";
import { projected, projectedRows } from "./validation.ts";
import { discovery } from "./discovery.ts";
import { live } from "./stream.ts";
import { journalBatch } from "./journal.ts";
import { workResponse } from "./work-validation.ts";
import { acceptedAnchor, anchorRequest, queryWire } from "./query.ts";
import type {
  ClientOptions,
  Invocation,
  PendingMutation,
  RomClient,
  QuerySpec,
  WireObject,
  WireValue,
} from "./types.ts";

export function createClient(options: ClientOptions): RomClient {
  for (const [name, value] of Object.entries({
    maxBytes: options.maxBytes ?? 1048576,
    timeoutMs: options.timeoutMs ?? 15000,
    maxRows: options.maxRows ?? 10000,
    maxObservations: options.maxObservations ?? 8,
  })) {
    if (
      !Number.isSafeInteger(value) ||
      value < 1 ||
      (name === "timeoutMs" && value > 2147483647)
    )
      throw new Error(`invalid ${name}`);
  }
  let generation = 0;
  let observations = 0;
  const active = new Set<AbortController>();
  const prepared = new WeakMap<
    PendingMutation,
    {
      body: Invocation;
      fingerprint: string;
      generation: number;
      running: boolean;
      terminal: boolean;
    }
  >();
  async function run<T>(
    execute: (signal: AbortSignal) => Promise<T>,
    external?: AbortSignal,
  ): Promise<T> {
    const started = generation,
      controller = new AbortController();
    active.add(controller);
    const abort = () => controller.abort(external?.reason);
    if (external?.aborted) abort();
    else external?.addEventListener("abort", abort, { once: true });
    try {
      if (controller.signal.aborted) throw controller.signal.reason;
      const result = await deadline(
        execute(controller.signal),
        controller.signal,
        options.timeoutMs ?? 15000,
        () => controller.abort(new Error("request timeout")),
      );
      if (started !== generation) throw new Error("session changed");
      return result;
    } finally {
      external?.removeEventListener("abort", abort);
      active.delete(controller);
    }
  }
  function call(
    route: string,
    body: WireValue,
    external?: AbortSignal,
  ): Promise<WireValue> {
    return run((signal) => post(options, route, body, signal), external);
  }
  const client: RomClient = {
    ...blobClient(options, run, () => generation),
    get generation() {
      return generation;
    },
    invalidateSession() {
      generation++;
      for (const controller of active)
        controller.abort(new Error("session changed"));
    },
    async discover(signal) {
      return discovery(await call("discover", {}, signal));
    },
    async read(kind, id, signal) {
      return projected(await call("read", { kind, id }, signal), kind, id);
    },
    async query(kind, query, signal) {
      const limit = pageLimit(query, options.maxRows ?? 10000);
      return projectedRows(
        await call("query", { kind, query: queryWire(kind, query) }, signal),
        kind,
        limit,
      );
    },
    async anchor(query, last, signal) {
      const submitted = parseWire(
        stringifyWire(anchorRequest(query, last)),
      ) as WireObject;
      const view = submitted.view as WireObject,
        key = view.key as WireObject;
      return acceptedAnchor(
        await call("query/anchor", submitted, signal),
        key.kind as string,
        key.id as string,
        submitted.query as unknown as QuerySpec,
      );
    },
    prepare(request) {
      const fingerprint = stringifyWire(request as unknown as WireValue);
      const body = parseWire(fingerprint) as unknown as Invocation;
      const mutation: PendingMutation = {
        request: body,
        fingerprint,
        generation,
        state: "pending",
      };
      prepared.set(mutation, {
        body: parseWire(fingerprint) as unknown as Invocation,
        fingerprint,
        generation,
        running: false,
        terminal: false,
      });
      return mutation;
    },
    async submit(mutation, signal) {
      const entry = prepared.get(mutation);
      if (!entry) throw new Error("unrecognized mutation");
      if (entry.generation !== generation) throw new Error("session changed");
      if (entry.running) throw new Error("mutation already pending");
      if (entry.terminal) throw new Error("mutation is terminal");
      entry.running = true;
      mutation.state = "pending";
      delete mutation.result;
      delete mutation.error;
      try {
        const result = projected(
          await call("invoke", entry.body as unknown as WireValue, signal),
          entry.body.kind,
          entry.body.id,
        );
        mutationResult(entry.body, result);
        mutation.result = result;
        mutation.state = "succeeded";
        return result;
      } catch (error) {
        mutation.error =
          error instanceof Error ? error.message : "request failed";
        mutation.state =
          error instanceof RemoteError
            ? error.category === "conflict"
              ? "conflict"
              : error.category === "outcome_unknown" ||
                  (error.status >= 500 && error.category !== "not_committed")
                ? "unknown"
                : "rejected"
            : "unknown";
        entry.terminal =
          mutation.state === "rejected" || mutation.state === "conflict";
        throw error;
      } finally {
        entry.running = false;
      }
    },
    async *observe(kind, query, external) {
      if (external.aborted)
        throw external.reason ?? Error("observation aborted");
      if (observations >= (options.maxObservations ?? 8))
        throw Error("observation limit");
      const submitted = parseWire(
        stringifyWire(queryWire(kind, query)),
      ) as unknown as QuerySpec;
      pageLimit(submitted, options.maxRows ?? 10000);
      observations++;
      const started = generation,
        controller = new AbortController();
      active.add(controller);
      const abort = () => controller.abort(external.reason);
      if (external.aborted) abort();
      else external.addEventListener("abort", abort, { once: true });
      try {
        yield* live(
          options,
          kind,
          submitted,
          controller.signal,
          () => started === generation,
        );
      } finally {
        observations--;
        external.removeEventListener("abort", abort);
        active.delete(controller);
        controller.abort();
      }
    },
    async journal(kind, after = null, signal) {
      const checkpoint =
        after === null
          ? null
          : (parseWire(stringifyWire(after)) as typeof after);
      return journalBatch(
        await call("journal", { kind, after: checkpoint }, signal),
        kind,
        checkpoint,
        options.maxRows ?? 10000,
      );
    },
    async work(route, request, signal) {
      const submitted = parseWire(stringifyWire(request)) as typeof request;
      return workResponse(
        route,
        await call(`work/${route}`, submitted, signal),
        submitted,
        options.maxRows ?? 10000,
      );
    },
  };
  return client;
}
