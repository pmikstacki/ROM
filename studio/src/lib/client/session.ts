import { parseWire, stringifyWire } from "./codec.ts";
import { post, RemoteError } from "./request.ts";
import { projected, projectedRows } from "./validation.ts";
import { discovery } from "./discovery.ts";
import { live } from "./stream.ts";
import { journalBatch } from "./journal.ts";
import { workResponse } from "./work-validation.ts";
import type {
  ClientOptions,
  Invocation,
  PendingMutation,
  RomClient,
  WireValue,
} from "./types.ts";

export function createClient(options: ClientOptions): RomClient {
  for (const [name, value] of Object.entries({
    maxBytes: options.maxBytes ?? 1048576,
    timeoutMs: options.timeoutMs ?? 15000,
    maxRows: options.maxRows ?? 10000,
  })) {
    if (!Number.isSafeInteger(value) || value < 1)
      throw new Error(`invalid ${name}`);
  }
  let generation = 0;
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
  async function call(
    route: string,
    body: WireValue,
    external?: AbortSignal,
  ): Promise<WireValue> {
    const started = generation,
      controller = new AbortController();
    active.add(controller);
    const abort = () => controller.abort(external?.reason);
    if (external?.aborted) abort();
    else external?.addEventListener("abort", abort, { once: true });
    const timer = setTimeout(
      () => controller.abort(new Error("request timeout")),
      options.timeoutMs ?? 15000,
    );
    try {
      const result = await post(options, route, body, controller.signal);
      if (started !== generation) throw new Error("session changed");
      return result;
    } finally {
      clearTimeout(timer);
      external?.removeEventListener("abort", abort);
      active.delete(controller);
    }
  }
  const client: RomClient = {
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
      return projectedRows(
        await call("query", { kind, query } as unknown as WireValue, signal),
        kind,
        options.maxRows ?? 10000,
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
          query,
          controller.signal,
          () => started === generation,
        );
      } finally {
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
