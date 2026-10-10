import { withDeadline, signalBody } from './http-lifetime.mjs';
import { loadWorkParser } from "./work-parser.mjs";
const base = "https://127.0.0.1:44389";
const paths = new Set([
  "/rom-studio/api/read",
  "/rom-studio/api/query",
  "/rom-studio/api/invoke",
  "/rom-studio/blobs/reserve",
  ...["capabilities", "list", "read", "control"].map(
    (route) => `/rom-studio/api/work/${route}`,
  ),
]);
function admitted(request) {
  if (
    !request ||
    !["GET", "POST"].includes(request.method) ||
    typeof request.path !== "string" ||
    !(
      paths.has(request.path) ||
      /^\/rom-studio\/blobs\/upload\?id=load-attachment-[0-9]{1,3}$/.test(
        request.path,
      ) ||
      /^\/rom-studio\/blobs\/attachment\/load-attachment-[0-9]{1,3}$/.test(
        request.path,
      )
    )
  )
    throw Error("closed load HTTP request");
  if (
    request.path.startsWith("/rom-studio/api/work/") &&
    (request.operator !== true ||
      request.method !== "POST" ||
      request.measurement !== "work")
  )
    throw Error("explicit operator load request required");
}
function view(value, kind, id) {
  return (
    value?.key?.kind === kind &&
    (id === undefined || value.key.id === id) &&
    typeof value.key.id === "string" &&
    Number.isSafeInteger(value.revision) &&
    value.revision >= 1 &&
    value.value !== null &&
    typeof value.value === "object" &&
    !Array.isArray(value.value)
  );
}
export function httpExecutor(session, acquire = fetch) {
  if (
    !session ||
    Object.keys(session).length !== 2 ||
    typeof session.cookie !== "string" ||
    !/^rom_session=[A-Za-z0-9_-]{1,512}$/.test(session.cookie) ||
    typeof session.csrf !== "string" ||
    !/^[A-Za-z0-9_-]{1,512}$/.test(session.csrf) ||
    typeof acquire !== "function"
  )
    throw Error("private bounded load session");
  let requests = 0,
    bytes = 0;
  const failureClasses = {};
  const httpErrorCodes = {};
  function outcome(request, value, reason) {
    if (value !== "success") {
      const measurement = ["read", "query", "commit", "replay", "reserve", "upload", "download", "work"].includes(request.measurement) ? request.measurement : "other";
      const key = measurement + ":" + reason;
      failureClasses[key] = Math.min(13280, (failureClasses[key] ?? 0) + 1);
    }
    return value;
  }
  return {
    counts: () => ({
      requests,
      failure_classes: { ...failureClasses },
      http_error_codes: { ...httpErrorCodes },
      response_bytes: bytes,
      maximum_requests: 13280,
      maximum_response_bytes: 64 * 1024 * 1024,
    }),
    async execute(request, signal) {
      admitted(request);
      if (requests >= 13280) throw Error("finite HTTP request budget");
      requests++;
      const mutating =
        request.path === "/rom-studio/api/work/control" ||
        ["commit", "replay", "reserve", "upload"].includes(request.measurement);
      const headers = {
        cookie: session.cookie,
        "content-type": request.binary_bytes
          ? "application/octet-stream"
          : "application/json",
        ...(request.method === "POST"
          ? { origin: base, "x-rom-csrf": session.csrf }
          : {}),
      };
      return await withDeadline(5000,[signal],async deadline => {
      try {
        const body = request.binary_bytes
          ? Buffer.alloc(request.binary_bytes, 0x61)
          : request.body
            ? JSON.stringify(request.body)
            : undefined;
        if (body && Buffer.byteLength(body) > 65536)
          throw Error("bounded HTTP body");
        const response = await acquire(base + request.path, {
          method: request.method,
          headers,
          ...(body ? { body } : {}),
          redirect: "error",
          signal: deadline,
        });
        const chunks = [];
        let length = 0;
        if (response.body)
          for await (const chunk of signalBody(response.body,deadline)) {
            length += chunk.length;
            bytes += chunk.length;
            if (length > 65536 || bytes > 64 * 1024 * 1024) {
              await response.body.cancel().catch(() => {});
              throw Error("bounded HTTP response");
            }
            chunks.push(Buffer.from(chunk));
          }
        const received = Buffer.concat(chunks);
        if (response.status >= 400) {
          let code;
          try { code = JSON.parse(received.toString("utf8"))?.error; } catch {}
          const closed = ["denied", "missing", "conflict", "identity_mismatch", "identity_expired", "history_gap", "too_large", "overloaded", "closed", "outcome_unknown", "not_committed", "invalid", "internal", "timeout"].includes(code) ? code : "other";
          httpErrorCodes[closed] = Math.min(13280, (httpErrorCodes[closed] ?? 0) + 1);
        }
        if (response.status === 503) {
          let error;
          try {
            error = JSON.parse(received.toString("utf8")).error;
          } catch {}
          if (error === "outcome_unknown") return outcome(request, "unknown", "outcome-unknown");
        }
        if ([401, 403].includes(response.status)) return outcome(request, "denied", "http-denied");
        if (response.status === 429 || response.status >= 500)
          return outcome(request, "overloaded", response.status === 429 ? "http-429" : "http-5xx");
        if (response.status !== 200) return outcome(request, "failure", "http-status");

        if (request.expected_bytes)
          return received.length === request.expected_bytes &&
            received.every((value) => value === 0x61)
            ? "success"
            : "failure";
        if (request.path.startsWith("/rom-studio/api/work/")) {
          const parser = await loadWorkParser();
          const route = request.path.slice(request.path.lastIndexOf("/") + 1);
          const value = parser.workResponse(
            route,
            parser.parseWire(received.toString("utf8"), 65536),
            request.body ?? {},
            50,
          );
          return request.expected_capability &&
            value[request.expected_capability] !== true
            ? "denied"
            : "success";
        }
        const value = JSON.parse(received.toString("utf8"));
        if (request.measurement === "query")
          return Array.isArray(value) &&
            value.length <= 50 &&
            value.every((row) => view(row, "load-records"))
            ? "success"
            : "failure";
        const kind = ["reserve", "upload"].includes(request.measurement)
          ? "blobs"
          : request.body.kind;
        const id =
          request.body?.id ??
          new URL(base + request.path).searchParams.get("id");
        if (["reserve", "upload"].includes(request.measurement)) {
          const expectedStatus = request.measurement === "reserve" ? "reserved" : "attached";
          return outcome(request, value?.status === expectedStatus && view(value.resource, kind, id) ? "success" : "failure", "blob-envelope");
        }
        return outcome(request, view(value, kind, id) ? "success" : "failure", "resource-view");
      } catch (error) {
        if (
          signal.aborted ||
          error?.name === "TimeoutError" ||
          error?.name === "AbortError"
        )
          return outcome(request, "timeout", "transport-timeout");
        return outcome(request, mutating ? "unknown" : "failure", "transport-exception");
      }
      });
    },
  };
}
