import assert from "node:assert/strict";
import { createServer } from "node:http";
import { spawn } from "node:child_process";
import {
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync,
  existsSync,
} from "node:fs";
import { join } from "node:path";
import { startPortalFixture } from "../../http.mjs";

const binary = process.env.ROM_UI_HOST;
const adapter = process.env.ROM_UI_ADAPTER;
const evidence = process.env.ROM_UI_BIND_EVIDENCE;
if (!binary || !["sqlite", "redb"].includes(adapter) || !evidence)
  throw Error("explicit bind regression fixture required");
mkdirSync(evidence, { recursive: true });
const port = Number(process.env.ROM_UI_PORT ?? 43301);
const occupied = createServer((_, response) =>
  response.end("owned occupied listener"),
);
await new Promise((resolve, reject) => {
  occupied.once("error", reject);
  occupied.listen(port, "127.0.0.1", resolve);
});
const owned = new Set();
const children = () =>
  readFileSync(`/proc/${process.pid}/task/${process.pid}/children`, "utf8")
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .map(Number);
const alive = (pid) => existsSync(`/proc/${pid}`);
const monitor = setInterval(
  () => children().forEach((pid) => owned.add(pid)),
  5,
);
const waitFor = async (check, ms = 5000) => {
  const end = Date.now() + ms;
  while (!check()) {
    if (Date.now() > end) throw Error("bounded bind regression deadline");
    await new Promise((r) => setTimeout(r, 10));
  }
};
let failure, reopened;
const record = {
  adapter,
  port,
  owned_pids: [],
  bind_error: null,
  child_terminal_before_cleanup: false,
  reopened: false,
};
try {
  try {
    await startPortalFixture({
      binary,
      adapter,
      database: join(evidence, "database"),
      evidence,
      port,
      dist: evidence,
    });
    throw Error("occupied port unexpectedly admitted");
  } catch (error) {
    failure = error;
  }
  record.bind_error = failure.code;
  assert.equal(failure.code, "EADDRINUSE");
  clearInterval(monitor);
  children().forEach((pid) => owned.add(pid));
  record.owned_pids = [...owned];
  assert.equal(owned.size, 1, "exactly one fixture child launched before bind");
  record.child_terminal_before_cleanup = [...owned].every((pid) => !alive(pid));
  assert.equal(
    record.child_terminal_before_cleanup,
    true,
    "bind failure must drain the owned native child before rejecting",
  );
  const ready = join(evidence, "reopen.ready.json"),
    stop = join(evidence, "reopen.stop");
  reopened = spawn(
    binary,
    ["--fixture-only", adapter, join(evidence, "database.case-1"), ready, stop],
    { stdio: ["ignore", "ignore", "ignore"] },
  );
  let exit;
  reopened.once("exit", (code) => (exit = code));
  await waitFor(() => existsSync(ready) || exit !== undefined);
  assert.equal(
    exit,
    undefined,
    "same database must reopen after failed bind cleanup",
  );
  const address = JSON.parse(readFileSync(ready, "utf8")).base_url;
  const response = await fetch(address + "/read", {
    method: "POST",
    headers: {
      authorization: "Bearer fixture-alice",
      "content-type": "application/json",
    },
    body: JSON.stringify({ kind: "portal-settings", id: "workspace" }),
    signal: AbortSignal.timeout(5000),
  });
  assert.equal(response.status, 200);
  assert.equal((await response.json()).key.id, "workspace");
  writeFileSync(stop, "stop\n");
  await waitFor(() => exit !== undefined);
  assert.equal(exit, 0);
  record.reopened = true;
} finally {
  clearInterval(monitor);
  // Cleanup is restricted to this regression's private paths and observed children.
  for (const file of readdirSync(evidence).filter((file) =>
    file.endsWith(".ready.json"),
  ))
    writeFileSync(
      join(evidence, file.replace(/\.ready\.json$/, ".stop")),
      "stop\n",
    );
  await waitFor(() => [...owned].every((pid) => !alive(pid))).catch(() => {});
  if (reopened && alive(reopened.pid)) {
    writeFileSync(join(evidence, "reopen.stop"), "stop\n");
    await waitFor(() => !alive(reopened.pid));
  }
  await new Promise((resolve) => occupied.close(resolve));
  writeFileSync(join(evidence, "result.json"), JSON.stringify(record, null, 2));
}
console.log(JSON.stringify(record));
