import { spawnSync } from "node:child_process";
import {
  readFileSync,
  writeFileSync,
  readlinkSync,
  realpathSync,
  lstatSync,
} from "node:fs";
import { mountAcknowledged } from "../identity/provider/mount-ack.mjs";
import {
  namespaceTools,
  requirePrivateTrustNamespace,
} from "../identity/provider/trust-namespace.mjs";
import { readProcessIdentity } from "../identity/provider/launch.mjs";
const value = JSON.parse(process.argv[2]),
  prefix =
    "/var/tmp/rom-010-authentik-20261007/run/volume/private/application-recovery-";
for (const path of [value.ack.path, value.result, value.source, value.empty])
  if (!path.startsWith(prefix) || path.includes("/../"))
    throw Error("owned recovery mount paths required");
for (const path of [value.source, value.empty])
  if (
    realpathSync(path) !== path ||
    !lstatSync(path).isDirectory() ||
    lstatSync(path).uid !== process.getuid()
  )
    throw Error("canonical owned mount paths");
const ns = readlinkSync("/proc/self/ns/mnt");
if (
  value.namespace !== ns ||
  value.membership !== readFileSync("/proc/self/cgroup", "utf8").trim()
)
  throw Error("current owned mount namespace required");
requirePrivateTrustNamespace(
  value.parent_namespace,
  ns,
  readFileSync("/proc/self/mountinfo", "utf8"),
);
const commands =
  value.operation === "bind"
    ? ["--bind", value.empty, value.source]
    : value.operation === "readonly"
      ? ["--bind", "-o", "remount,ro", value.source]
      : null;
if (!commands) throw Error("closed recovery mount operation required");
const end = Date.now() + 4000;
while (!mountAcknowledged(value.ack)) {
  if (Date.now() >= end) throw Error("mount admission deadline");
  await new Promise((r) => setTimeout(r, 10));
}
const result = spawnSync(namespaceTools.mount, commands, {
  timeout: 5000,
  stdio: "ignore",
});
writeFileSync(
  value.result,
  JSON.stringify({
    bridge_birth: readProcessIdentity(process.pid),
    namespace: ns,
    operation: value.operation,
    exit_code: result.status,
    tool_pid: result.pid,
    tool_pid_continuous_birth_admission: false,
  }),
  { flag: "wx", mode: 0o600 },
);
process.exitCode = result.status === 0 ? 0 : 1;
