// Disposable loopback fixture proxy. Faults affect transport after real backend work.
import { createServer } from "node:http";
import { spawn } from "node:child_process";
import {
  createReadStream,
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  appendFileSync,
  statSync,
} from "node:fs";
import { resolve, join, extname } from "node:path";
import { Readable } from "node:stream";

export async function startPortalFixture({
  binary,
  adapter,
  database,
  evidence,
  port,
  dist,
}) {
  if (!["sqlite", "redb"].includes(adapter))
    throw Error("invalid fixture adapter");
  mkdirSync(evidence, { recursive: true });
  let backend,
    ready,
    caseNumber = 0,
    currentDatabase,
    fault = null,
    closed = false;
  let recordedBytes = 0,
    requestBytes = 0,
    errorBytes = 0,
    controlCount = 0,
    queuedControls = 0;
  let controlTail = Promise.resolve();
  const held = new Set();
  const requests = [];
  function reportError(error) {
    const text = String(error).slice(0, 1024) + "\n",
      length = Buffer.byteLength(text);
    if (errorBytes + length > 65536) return;
    errorBytes += length;
    appendFileSync(join(evidence, "proxy-errors.log"), text);
  }
  async function waitFor(check, limit = 10000) {
    const deadline = Date.now() + limit;
    while (!check()) {
      if (Date.now() >= deadline) throw Error("fixture deadline");
      await new Promise((r) => setTimeout(r, 10));
    }
  }
  async function stopBackend() {
    if (!backend) return;
    const current = backend;
    writeFileSync(current.stop, "stop\n");
    await waitFor(() => current.exit !== undefined, 5000).catch(
      async (error) => {
        current.process.kill("SIGTERM");
        throw error;
      },
    );
    if (current.exit !== 0)
      throw Error("fixture backend shutdown failed: " + current.exit);
    backend = null;
  }
  async function launch(fresh) {
    await stopBackend();
    if (fresh) {
      caseNumber++;
      currentDatabase = database + ".case-" + caseNumber;
    }
    const prefix = join(evidence, "native-" + caseNumber + "-" + Date.now());
    const child = spawn(
      binary,
      [
        "--fixture-only",
        adapter,
        currentDatabase,
        prefix + ".ready.json",
        prefix + ".stop",
      ],
      { stdio: ["ignore", "pipe", "pipe"] },
    );
    const running = {
      process: child,
      stop: prefix + ".stop",
      exit: undefined,
      bytes: 0,
    };
    backend = running;
    const capture = (chunk) => {
      running.bytes += chunk.length;
      if (running.bytes > 1048576) {
        child.kill("SIGTERM");
        return;
      }
      appendFileSync(prefix + ".log", chunk);
    };
    child.stdout.on("data", capture);
    child.stderr.on("data", capture);
    child.once("error", () => {
      running.exit = -1;
    });
    child.once("exit", (code) => {
      running.exit = code ?? -1;
    });
    await waitFor(
      () => existsSync(prefix + ".ready.json") || running.exit !== undefined,
    );
    if (running.exit !== undefined)
      throw Error("fixture backend exited before ready");
    if (statSync(prefix + ".ready.json").size > 4096)
      throw Error("fixture readiness byte limit");
    ready = JSON.parse(readFileSync(prefix + ".ready.json", "utf8"));
    const parsed = new URL(ready.base_url);
    const nativePort = Number(parsed.port);
    if (
      ready.lane !== "fixture-only" ||
      parsed.protocol !== "http:" ||
      parsed.hostname !== "127.0.0.1" ||
      !Number.isSafeInteger(nativePort) ||
      nativePort < 1024 ||
      nativePort > 65535 ||
      parsed.username ||
      parsed.password ||
      parsed.pathname !== "/" ||
      parsed.search ||
      parsed.hash ||
      ready.address !== "127.0.0.1:" + nativePort ||
      ready.database !== adapter
    )
      throw Error("unexpected fixture readiness");
    writeFileSync(
      prefix + ".ready-reviewed.json",
      JSON.stringify({ ...ready, database_path: currentDatabase }, null, 2),
    );
  }
  async function body(request, limit = 1048576) {
    const chunks = [];
    let size = 0;
    for await (const chunk of request) {
      size += chunk.length;
      if (size > limit) throw Error("fixture request limit");
      chunks.push(chunk);
    }
    return Buffer.concat(chunks);
  }
  function release() {
    for (const resume of held) resume();
    held.clear();
  }
  try {
    await launch(true);
  } catch (error) {
    await stopBackend().catch(() => {});
    throw error;
  }
  const server = createServer(async (request, response) => {
    try {
      const path = new URL(request.url, "http://localhost").pathname;
      if (path === "/__fixture/control") {
        const command = JSON.parse((await body(request, 4096)).toString());
        if (
          !command ||
          typeof command !== "object" ||
          Array.isArray(command) ||
          Object.keys(command).some(
            (key) => !["type", "route", "kind"].includes(key),
          ) ||
          ![
            "reset",
            "restart",
            "release",
            "drop-ack",
            "hold",
            "unavailable",
          ].includes(command.type) ||
          (command.route !== undefined &&
            ![
              "invoke",
              "read",
              "query",
              "live",
              "journal",
              "discover",
            ].includes(command.route)) ||
          (command.kind !== undefined &&
            ![
              "equipment",
              "inspections",
              "work-orders",
              "portal-settings",
              "maintenance-guides",
            ].includes(command.kind))
        )
          throw Error("invalid fixture control");
        if (++controlCount > 128 || queuedControls >= 4)
          throw Error("fixture control admission limit");
        queuedControls++;
        const admitted = controlTail.then(async () => {
          if (command.type === "reset") {
            release();
            fault = null;
            requests.length = 0;
            requestBytes = 0;
            await launch(true);
          } else if (command.type === "restart") {
            release();
            fault = null;
            await launch(false);
          } else if (command.type === "release") release();
          else fault = command;
        });
        controlTail = admitted.catch(() => {}).finally(() => queuedControls--);
        await admitted;
        response.writeHead(200, { "content-type": "application/json" });
        response.end(JSON.stringify({ okay: true }));
        return;
      }
      if (path === "/__fixture/evidence") {
        response.writeHead(200, { "content-type": "application/json" });
        response.end(
          JSON.stringify({ adapter, ready, requests, held: held.size }),
        );
        return;
      }
      if (path.startsWith("/api/")) {
        const bytes = await body(request),
          route = path.slice(5),
          submitted = JSON.parse(bytes.toString());
        const matched =
          fault &&
          (!fault.route || fault.route === route) &&
          (!fault.kind || fault.kind === submitted.kind);
        const selectedFault = matched ? fault : null;
        if (matched) fault = null;
        if (selectedFault?.type === "unavailable") {
          response.writeHead(503, { "content-type": "application/json" });
          response.end('{"error":"closed"}');
          return;
        }
        const record = {
          route,
          body: bytes.toString(),
          subject:
            request.headers.authorization === undefined
              ? "public-guest"
              : request.headers.authorization === "Bearer fixture-alice"
                ? "alice"
                : request.headers.authorization === "Bearer fixture-bob"
                  ? "bob"
                  : "unverified",
          authorization_present: request.headers.authorization !== undefined,
        };
        const encoded = JSON.stringify(record) + "\n",
          length = Buffer.byteLength(encoded);
        if (
          requests.length >= 256 ||
          requestBytes + length > 1048576 ||
          recordedBytes + length > 4194304
        )
          throw Error("fixture evidence budget");
        requests.push(record);
        requestBytes += length;
        recordedBytes += length;
        appendFileSync(join(evidence, "http-requests.jsonl"), encoded);
        const abort = new AbortController();
        response.once("close", () => abort.abort());
        const upstream = await fetch(ready.base_url + "/" + route, {
          method: "POST",
          headers: {
            "content-type": "application/json",
            ...(request.headers.authorization === undefined
              ? {}
              : { authorization: request.headers.authorization }),
            "x-subject": "alice",
          },
          body: bytes,
          signal:
            route === "live" || route === "subscribe"
              ? abort.signal
              : AbortSignal.any([abort.signal, AbortSignal.timeout(10000)]),
        });
        if (route === "live" || route === "subscribe") {
          response.writeHead(upstream.status, {
            "content-type":
              upstream.headers.get("content-type") ?? "application/json",
          });
          if (upstream.body)
            Readable.fromWeb(upstream.body)
              .on("error", () => response.destroy())
              .pipe(response);
          else response.end();
          return;
        }
        const reader = upstream.body?.getReader(),
          chunks = [];
        let size = 0;
        if (reader)
          try {
            for (;;) {
              const { done, value } = await reader.read();
              if (done) break;
              size += value.length;
              if (size > 1048576) throw Error("fixture upstream limit");
              chunks.push(Buffer.from(value));
            }
          } finally {
            await reader.cancel().catch(() => {});
          }
        const result = Buffer.concat(chunks);
        if (
          selectedFault?.type === "drop-ack" &&
          route === "invoke" &&
          upstream.ok
        ) {
          response.destroy();
          return;
        }
        if (selectedFault?.type === "hold") {
          await new Promise((resolve) => {
            const resume = () => {
              clearTimeout(timer);
              held.delete(resume);
              resolve();
            };
            const timer = setTimeout(resume, 15000);
            held.add(resume);
            response.once("close", resume);
          });
        }
        if (response.destroyed) return;
        response.writeHead(upstream.status, {
          "content-type":
            upstream.headers.get("content-type") ?? "application/json",
          "content-length": result.length,
        });
        response.end(result);
        return;
      }
      const file = resolve(dist, "." + (path === "/" ? "/index.html" : path));
      if (
        !file.startsWith(resolve(dist) + "/") ||
        !existsSync(file) ||
        !statSync(file).isFile()
      ) {
        response.writeHead(404);
        response.end();
        return;
      }
      const types = {
        ".html": "text/html",
        ".js": "text/javascript",
        ".css": "text/css",
        ".woff2": "font/woff2",
        ".svg": "image/svg+xml",
      };
      response.writeHead(200, {
        "content-type": types[extname(file)] ?? "application/octet-stream",
      });
      createReadStream(file).pipe(response);
    } catch (error) {
      if (response.destroyed) return;
      response.writeHead(500, { "content-type": "application/json" });
      response.end('{"error":"fixture_failure"}');
      reportError(error);
    }
  });
  server.maxConnections = 32;
  server.requestTimeout = 10000;
  server.headersTimeout = 10000;
  try {
    await new Promise((resolve, reject) => {
      server.once("error", reject);
      server.listen(port, "127.0.0.1", resolve);
    });
  } catch (error) {
    release();
    server.closeAllConnections();
    await stopBackend();
    throw error;
  }
  return {
    base: "http://127.0.0.1:" + port,
    async close() {
      if (closed) return;
      closed = true;
      release();
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
      await stopBackend();
    },
  };
}

if (
  process.env.ROM_UI_HOST &&
  process.argv[1] &&
  resolve(process.argv[1]) === resolve(new URL(import.meta.url).pathname)
) {
  const server = await startPortalFixture({
    binary: process.env.ROM_UI_HOST,
    adapter: process.env.ROM_UI_ADAPTER,
    database: process.env.ROM_UI_DATABASE,
    evidence: process.env.ROM_UI_EVIDENCE,
    port: Number(process.env.ROM_UI_PORT),
    dist: resolve("consumer/dist"),
  });
  for (const signal of ["SIGINT", "SIGTERM"])
    process.once(signal, () => void server.close().then(() => process.exit()));
  console.log(server.base);
}
