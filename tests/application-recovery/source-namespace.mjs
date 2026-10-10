import {
  readFileSync,
  writeFileSync,
  readlinkSync,
  lstatSync,
  readdirSync,
  realpathSync,
  openSync,
  fstatSync,
  closeSync,
  constants,
} from "node:fs";
import { randomBytes } from "node:crypto";
import { fileURLToPath } from "node:url";
import {
  OwnedLauncher,
  readProcessIdentity,
  sameProcessIdentity,
} from "../identity/provider/launch.mjs";
import { requirePrivateTrustNamespace } from "../identity/provider/trust-namespace.mjs";
export async function hideOriginalSource(value) {
  const ns = readlinkSync("/proc/self/ns/mnt"),
    membership = readFileSync("/proc/self/cgroup", "utf8").trim();
  requirePrivateTrustNamespace(
    value.parent_namespace,
    ns,
    readFileSync("/proc/self/mountinfo", "utf8"),
  );
  const prefix =
    "/var/tmp/rom-010-authentik-20261007/run/volume/private/application-recovery-";
  for (const path of [value.source, value.empty, value.private_root]) {
    const s = lstatSync(path);
    if (
      !path.startsWith(prefix) ||
      realpathSync(path) !== path ||
      !s.isDirectory() ||
      s.uid !== process.getuid() ||
      (s.mode & 0o777) !== 0o700
    )
      throw Error("private recovery source namespace required");
  }
  if (readdirSync(value.empty).length)
    throw Error("empty source concealment view required");
  const before = lstatSync(value.source),
    empty = lstatSync(value.empty),
    launcher = new OwnedLauncher(),
    record = {
      parent_namespace: value.parent_namespace,
      namespace: ns,
      source_original_inode: before.ino,
      empty_inode: empty.ino,
      mounts: [],
    };
  try {
    for (const operation of ["bind", "readonly"]) {
      const nonce = randomBytes(12).toString("hex"),
        path = value.private_root + "/mount-ack-" + nonce,
        result = value.private_root + "/mount-result-" + nonce + ".json";
      writeFileSync(path, "", { flag: "wx", mode: 0o600 });
      const s = lstatSync(path),
        ack = { path, device: s.dev, inode: s.ino, uid: s.uid };
      const child = launcher.launch(
        process.execPath,
        [
          fileURLToPath(new URL("./source-mount-run.mjs", import.meta.url)),
          JSON.stringify({
            source: value.source,
            empty: value.empty,
            ack,
            result,
            operation,
            namespace: ns,
            parent_namespace: value.parent_namespace,
            membership,
          }),
        ],
        {
          timeoutMs: 10000,
          graceMs: 1000,
          drainMs: 1000,
          outputBytes: 65536,
          admitIdentity: (id) => {
            if (
              readlinkSync(`/proc/${id.pid}/ns/mnt`) !== ns ||
              readFileSync(`/proc/${id.pid}/cgroup`, "utf8").trim() !==
                membership ||
              !sameProcessIdentity(id, readProcessIdentity(id.pid))
            )
              throw Error("mount child namespace ownership changed");
          },
        },
      );
      child.child.stdout.on("data", () => {});
      child.child.stderr.on("data", () => {});
      const target = await child.targetStarted;
      if (target) {
        const fd = openSync(path, constants.O_WRONLY | constants.O_NOFOLLOW);
        try {
          const current = fstatSync(fd);
          if (
            current.ino !== s.ino ||
            current.dev !== s.dev ||
            current.size !== 0
          )
            throw Error("mount admission changed");
          writeFileSync(fd, "ready\n");
        } finally {
          closeSync(fd);
        }
      }
      const closed = await child.closed;
      await child.physicalClose;
      const command = JSON.parse(readFileSync(result));
      record.mounts.push({ wrapper: child.identity, target, closed, command });
      if (
        !target ||
        closed.exit_code !== 0 ||
        !closed.drained ||
        command.exit_code !== 0
      )
        throw Error("source concealment mount failed");
    }
    const mounted = lstatSync(value.source),
      mount = readFileSync("/proc/self/mountinfo", "utf8")
        .split("\n")
        .filter((line) => line.split(" ")[4] === value.source);
    if (
      mounted.ino !== empty.ino ||
      mounted.ino === before.ino ||
      mount.length !== 1 ||
      !mount[0].split(" ")[5].split(",").includes("ro") ||
      readdirSync(value.source).length
    )
      throw Error("source isolation projection failed");
    record.source_hidden = true;
    record.readonly = true;
    record.mountinfo = mount;
    return record;
  } finally {
    record.drain = await launcher.drain();
    writeFileSync(value.namespace_result, JSON.stringify(record, null, 2), {
      flag: "wx",
      mode: 0o600,
    });
  }
}
