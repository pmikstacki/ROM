import assert from "node:assert/strict";
import { createServer } from "node:http";
import { httpExecutor } from "./http-executor.mjs";

const server = createServer((request, response) => {
  response.writeHead(200, { "content-type": "application/json" });
  response.write("{");
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const gc = setInterval(() => global.gc(), 50);
try {
  const executor = httpExecutor(
    { cookie: "rom_session=fixture", csrf: "fixture" },
    (url, options) =>
      fetch(`http://127.0.0.1:${server.address().port}/read`, options),
  );
  const started = performance.now();
  const outcome = await executor.execute(
    {
      method: "POST",
      path: "/rom-studio/api/read",
      measurement: "read",
      body: { kind: "load-records", id: "load-00000" },
    },
    new AbortController().signal,
  );
  const elapsed = performance.now() - started;
  assert.equal(outcome, "timeout");
  assert.ok(elapsed >= 4900);
  assert.ok(elapsed < 8000);
  console.log(JSON.stringify({ outcome, elapsed_ms: elapsed }));
} finally {
  clearInterval(gc);
  const closed = new Promise((resolve) => server.close(resolve));
  server.closeAllConnections();
  await closed;
}
