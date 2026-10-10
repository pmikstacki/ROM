import { readFileSync, writeFileSync, readlinkSync } from "node:fs";
import {
  OwnedLauncher,
  readProcessIdentity,
} from "../identity/provider/launch.mjs";
import { hideOriginalSource } from "./source-namespace.mjs";
const c = JSON.parse(readFileSync(process.argv[2]));
await hideOriginalSource(c);
const launcher = new OwnedLauncher();
let record;
try {
  const child = launcher.launch(c.binary, [c.host_configuration], {
    timeoutMs: c.mode === "serve" ? 95000 : 60000,
    graceMs: 1000,
    drainMs: 1000,
    outputBytes: 65536,
    admitIdentity: (id) => {
      if (
        readlinkSync(`/proc/${id.pid}/ns/mnt`) !==
        readlinkSync("/proc/self/ns/mnt")
      )
        throw Error("recovered child lost source isolation");
    },
  });
  child.child.stdout.on("data", (bytes) => process.stdout.write(bytes));
  child.child.stderr.on("data", (bytes) => process.stderr.write(bytes));
  const target = await child.targetStarted;
  record = {
    target,
    wrapper: child.identity,
    bridge: readProcessIdentity(process.pid),
  };
  if (c.process_started)
    writeFileSync(c.process_started, JSON.stringify(record), {
      flag: "wx",
      mode: 0o600,
    });
  record.closed = await child.closed;
  await child.physicalClose;
  process.exitCode = record.closed.exit_code === 0 ? 0 : 1;
} finally {
  record = { ...record, drain: await launcher.drain() };
  writeFileSync(c.process_result, JSON.stringify(record, null, 2), {
    flag: "wx",
    mode: 0o600,
  });
}
