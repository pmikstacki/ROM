import { OwnedLauncher } from "../identity/provider/launch.mjs";
import {
  createProviderCgroup,
  admitOwnedWrapper,
  snapshotProviderCgroup,
  requireDrainedCgroup,
} from "../identity/provider/cgroup.mjs";
export async function runOwned(
  executable,
  args,
  { timeoutMs = 60000, env = process.env, onStart } = {},
) {
  const launcher = new OwnedLauncher(),
    group = createProviderCgroup(),
    record = { stdout: "", stderr: "" };
  let child;
  try {
    child = launcher.launch(executable, args, {
      timeoutMs,
      graceMs: 1000,
      drainMs: 1000,
      outputBytes: 65536,
      env,
      admitIdentity: (id) => admitOwnedWrapper(group, id),
    });
    child.child.stdout.on("data", (v) => {
      if (record.stdout.length < 65536) record.stdout += v.toString();
    });
    child.child.stderr.on("data", (v) => {
      if (record.stderr.length < 65536) record.stderr += v.toString();
    });
    record.wrapper = child.identity;
    record.target = await child.targetStarted;
    if (onStart) await onStart(record, child);
    record.closed = await child.closed;
    await child.physicalClose;
    return record;
  } finally {
    record.drain = await launcher.drain();
    record.group = snapshotProviderCgroup(group);
    requireDrainedCgroup(record.group.cgroup_events);
  }
}
