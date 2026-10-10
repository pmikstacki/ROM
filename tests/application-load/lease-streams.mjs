import { performance } from "node:perf_hooks";
import { withDeadline, signalBody } from "./http-lifetime.mjs";
import { serverTerminalCategory } from "./server-terminal-category.mjs";
const origin = "https://127.0.0.1:44389";
const codes = new WeakMap();
const fail = (code) => {
  const error = new Error("lease observation refused");
  codes.set(error, code);
  return error;
};
const opaque = (value) =>
  typeof value === "string" && /^[A-Za-z0-9_-]{1,512}$/.test(value);
export async function openLeaseStreams(
  session,
  { normal = 4, slow = 1 } = {},
  acquire = fetch,
  { signal } = {},
) {
  if (
    !session ||
    !/^rom_session=[A-Za-z0-9_-]{1,512}$/.test(session.cookie) ||
    !opaque(session.csrf) ||
    !Number.isSafeInteger(normal) ||
    normal < 1 ||
    normal > 4 ||
    ![0, 1].includes(slow) ||
    typeof acquire !== "function" ||
    (signal !== undefined && !(signal instanceof AbortSignal))
  )
    throw fail("profile");
  const stop = new AbortController(),
    all = AbortSignal.any([stop.signal, ...(signal ? [signal] : [])]),
    controls = new Set(),
    loops = [];
  const started = performance.now(),
    ends = started + 150000,
    timer = setTimeout(() => {
      result.deadline_reached = true;
      stop.abort(new DOMException("Reader deadline", "TimeoutError"));
    }, 150000);
  const result = {
    schema: "rom-load-lease-streams-v2",
    normal,
    slow,
    bytes: 0,
    data_events: 0,
    error_events: 0,
    error_classes: {},
    expected_expiries: 0,
    terminal_eofs: 0,
    reacquisition_attempts: 0,
    recoveries: 0,
    fresh_snapshots: 0,
    initial_snapshots: 0,
    abandoned_recoveries: 0,
    cancelled_recoveries: 0,
    slow_reads: 0,
    refused: 0,
    session_bytes: 0,
    recovery_latency_ms: [],
    observation_gap_ms: [],
    gap_semantics: "terminal-eof-to-first-fresh-snapshot",
    additional_http: { session_requests: 0, live_requests: 0 },
    maximum_bytes: 4194304,
    maximum_recoveries_per_reader: 5,
    maximum_ms: 150000,
    deadline_reached: false,
  };
  let baseline,
    pending = 0;
  function time() {
    const value = performance.now();
    if (!Number.isFinite(value) || value < started || value > ends)
      throw fail("clock-or-deadline");
    return value;
  }
  function error(e) {
    const code = codes.get(e) ?? "transport";
    result.error_events++;
    result.error_classes[code] = (result.error_classes[code] ?? 0) + 1;
  }
  async function request(url, options, ownedSignal) {
    all.throwIfAborted();
    time();
    pending++;
    try {
      const response = await acquire(url, {
        ...options,
        redirect: "error",
        signal: ownedSignal,
      });
      if (all.aborted || ownedSignal.aborted) {
        await response.body?.cancel().catch(() => {});
        all.throwIfAborted();
        ownedSignal.throwIfAborted();
      }
      return response;
    } finally {
      pending--;
    }
  }
  async function current(ownedSignal) {
    if (result.additional_http.session_requests >= 21)
      throw fail("session-budget");
    result.additional_http.session_requests++;
    const response = await request(
      origin + "/rom-studio/auth/session",
      { method: "GET", headers: { cookie: session.cookie } },
      ownedSignal,
    );
    try {
      if (
        response.status !== 200 ||
        !response.body ||
        !response.headers.get("content-type")?.startsWith("application/json")
      )
        throw fail("session-refused");
      const chunks = [];
      let size = 0;
      for await (const chunk of signalBody(response.body, ownedSignal)) {
        size += chunk.byteLength;
        result.session_bytes += chunk.byteLength;
        if (size > 16384 || result.session_bytes > 344064)
          throw fail("session-byte-budget");
        chunks.push(chunk);
      }
      let value;
      try {
        value = JSON.parse(Buffer.concat(chunks).toString("utf8"));
      } catch {
        throw fail("session-shape");
      }
      const wall = Date.now();
      if (!Number.isSafeInteger(wall) || wall < 0) throw fail("wall-clock");
      if (
        value.authenticated !== true ||
        typeof value.user_id !== "string" ||
        value.user_id.length < 1 ||
        value.user_id.length > 512 ||
        typeof value.generation !== "string" ||
        value.generation.length < 1 ||
        value.generation.length > 512 ||
        !opaque(value.csrf_token) ||
        !Number.isSafeInteger(value.expires_at) ||
        value.expires_at <= Math.floor(wall / 1000)
      )
        throw fail("session-shape");
      if (
        baseline &&
        (value.user_id !== baseline.user_id ||
          value.generation !== baseline.generation ||
          value.expires_at !== baseline.expires_at)
      )
        throw fail("session-binding");
      baseline ??= {
        user_id: value.user_id,
        generation: value.generation,
        expires_at: value.expires_at,
      };
      return value.csrf_token;
    } finally {
      if (!response.body?.locked) await response.body?.cancel().catch(() => {});
    }
  }
  async function open(index, csrf, ownedSignal) {
    if (result.additional_http.live_requests >= 25)
      throw fail("acquisition-budget");
    result.additional_http.live_requests++;
    const response = await request(
      origin + "/rom-studio/api/live",
      {
        method: "POST",
        headers: {
          cookie: session.cookie,
          origin,
          "x-rom-csrf": csrf,
          "content-type": "application/json",
        },
        body: JSON.stringify({
          kind: "load-records",
          query: {
            filters: [
              {
                field: "title",
                value: `Synthetic load row ${String(index).padStart(5, "0")}`,
              },
            ],
            limit: 1,
          },
        }),
      },
      ownedSignal,
    );
    if (
      response.status !== 200 ||
      !response.body ||
      !response.headers.get("content-type")?.startsWith("text/event-stream")
    ) {
      result.refused++;
      await response.body?.cancel().catch(() => {});
      throw fail("stream-refused");
    }
    return response;
  }
  async function cycle(response, index, ownedSignal, onSnapshot) {
    let pendingFrame = "",
      expired = false,
      dataSeen = false;
    const decoder = new TextDecoder("utf-8", { fatal: true });
    for await (const chunk of signalBody(response.body, ownedSignal)) {
      result.bytes += chunk.byteLength;
      if (result.bytes > 4194304) throw fail("byte-budget");
      try {
        pendingFrame += decoder.decode(chunk, { stream: true });
      } catch {
        throw fail("frame-shape");
      }
      if (pendingFrame.length > 65536) throw fail("frame-budget");
      let boundary;
      while ((boundary = pendingFrame.indexOf("\n\n")) !== -1) {
        const frame = pendingFrame.slice(0, boundary);
        pendingFrame = pendingFrame.slice(boundary + 2);
        if (expired) throw fail("post-expiry-frame");
        const lines = frame.split("\n");
        if (lines.every((line) => line.startsWith(":"))) continue;
        if (
          lines.filter((line) => line.startsWith("event:")).length !== 1 ||
          lines.some(
            (line) =>
              !line.startsWith(":") &&
              !line.startsWith("event:") &&
              !line.startsWith("data:"),
          )
        )
          throw fail("frame-fields");
        const type = /^event: ?([^\n]+)$/m.exec(frame)?.[1],
          fields = frame
            .split("\n")
            .filter((line) => line.startsWith("data:"))
            .map((line) => line.slice(5).trimStart())
            .join("\n");
        if (type === "error") {
          if (fields !== "identity_expired")
            throw fail(`server-terminal-${serverTerminalCategory(fields)}`);
          if (!dataSeen) throw fail("expiry-without-snapshot");
          expired = true;
          result.expected_expiries++;
          continue;
        }
        if (type !== "data") throw fail("event-type");
        let value;
        try {
          value = JSON.parse(fields);
        } catch {
          throw fail("data-shape");
        }
        if (
          !Array.isArray(value) ||
          value.length !== 1 ||
          value[0]?.key?.kind !== "load-records" ||
          value[0].key.id !== `load-${String(index).padStart(5, "0")}` ||
          !Number.isSafeInteger(value[0].revision) ||
          value[0].revision < 1 ||
          typeof value[0].value !== "object" ||
          value[0].value === null ||
          Array.isArray(value[0].value) ||
          value[0].value.title !==
            `Synthetic load row ${String(index).padStart(5, "0")}` ||
          !Number.isSafeInteger(value[0].value.counter) ||
          value[0].value.counter < 0 ||
          value[0].value.counter > 12000 ||
          typeof value[0].value.open !== "boolean"
        )
          throw fail("data-shape");
        result.data_events++;
        if (!dataSeen) {
          dataSeen = true;
          onSnapshot();
        }
      }
    }
    try {
      pendingFrame += decoder.decode();
    } catch {
      throw fail("frame-shape");
    }
    if (pendingFrame.length !== 0) throw fail("incomplete-frame");
    if (!expired) throw fail("unexpected-eof");
    result.terminal_eofs++;
    return time();
  }
  async function reader(index, response, controller, ready) {
    let attempts = 0,
      recovering = false,
      completed = false,
      recoveryStart,
      gapStart,
      recoveryTimer,
      leaseDeadline = new AbortController();
    let first = true;
    recoveryTimer = setTimeout(
      () =>
        leaseDeadline.abort(
          new DOMException("Snapshot deadline", "TimeoutError"),
        ),
      5000,
    );
    try {
      while (true) {
        const owned = AbortSignal.any([
          all,
          controller.signal,
          leaseDeadline.signal,
        ]);
        const eof = await cycle(response, index, owned, () => {
          clearTimeout(recoveryTimer);
          if (first) {
            first = false;
            result.initial_snapshots++;
            ready.resolve();
          }
          if (recovering && !completed) {
            const now = time();
            result.fresh_snapshots++;
            result.recoveries++;
            result.recovery_latency_ms.push(now - recoveryStart);
            result.observation_gap_ms.push(now - gapStart);
            completed = true;
            recovering = false;
          }
        });
        if (recovering && !completed) throw fail("missing-fresh-snapshot");
        if (attempts >= 5) throw fail("recovery-budget");
        all.throwIfAborted();
        attempts++;
        result.reacquisition_attempts++;
        recovering = true;
        completed = false;
        recoveryStart = time();
        gapStart = eof;
        leaseDeadline = new AbortController();
        recoveryTimer = setTimeout(
          () =>
            leaseDeadline.abort(
              new DOMException("Recovery deadline", "TimeoutError"),
            ),
          10000,
        );
        const renewalSignal = AbortSignal.any([
          all,
          controller.signal,
          leaseDeadline.signal,
        ]);
        const csrf = await current(renewalSignal);
        response = await open(index, csrf, renewalSignal);
      }
    } catch (e) {
      if (first) ready.reject(fail("initial-snapshot"));
      if (all.aborted || controller.signal.aborted) {
        if (recovering && !completed) result.cancelled_recoveries++;
      } else {
        if (recovering && !completed) result.abandoned_recoveries++;
        error(e);
      }
    } finally {
      clearTimeout(recoveryTimer);
      controls.delete(controller);
      if (!response.body?.locked) await response.body?.cancel().catch(() => {});
    }
  }
  try {
    const csrf = await withDeadline(5000, [all], current);
    for (let index = 0; index < normal + slow; index++) {
      const controller = new AbortController();
      controls.add(controller);
      const response = await withDeadline(
        5000,
        [all, controller.signal],
        (owned) => open(index, csrf, owned),
      );
      if (index >= normal) {
        all.throwIfAborted();
        loops.push(
          new Promise((resolve) =>
            all.addEventListener(
              "abort",
              async () => {
                await response.body.cancel().catch(() => {});
                controls.delete(controller);
                resolve();
              },
              { once: true },
            ),
          ),
        );
      } else {
        let resolve, reject;
        const ready = new Promise((yes, no) => {
          resolve = yes;
          reject = no;
        });
        loops.push(reader(index, response, controller, { resolve, reject }));
        await ready;
      }
    }
  } catch (e) {
    stop.abort();
    clearTimeout(timer);
    await Promise.allSettled(loops);
    throw fail(codes.get(e) ?? "acquisition");
  }
  function counts() {
    return structuredClone(result);
  }
  return {
    counts,
    async close() {
      stop.abort();
      clearTimeout(timer);
      let timeout;
      try {
        const drained = await Promise.race([
          Promise.allSettled(loops).then(() => true),
          new Promise((resolve) => {
            timeout = setTimeout(() => resolve(false), 5000);
          }),
        ]);
        return {
          ...counts(),
          readers_drained: drained && pending === 0 && controls.size === 0,
        };
      } finally {
        clearTimeout(timeout);
      }
    },
  };
}
