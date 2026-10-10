import test from "node:test";
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

test("real HTTP body deadline survives garbage collection after headers", { timeout: 10000 }, async () => {
  const { stdout, stderr } = await promisify(execFile)(
    process.execPath,
    ["--expose-gc", fileURLToPath(new URL("./http-lifetime-gc-fixture.mjs", import.meta.url))],
    { timeout: 9000, killSignal: "SIGKILL", maxBuffer: 65536 },
  );
  assert.equal(stderr, "");
  const result = JSON.parse(stdout);
  assert.equal(result.outcome, "timeout");
  assert.ok(result.elapsed_ms >= 4900 && result.elapsed_ms < 8000);
});
