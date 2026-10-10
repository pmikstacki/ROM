import {
  mkdirSync,
  readFileSync,
  writeFileSync,
  openSync,
  readSync,
  writeSync,
  closeSync,
  fstatSync,
  fsyncSync,
  constants,
} from "node:fs";
import { createHash, randomBytes } from "node:crypto";
import { OwnedLauncher, readProcessIdentity, sameProcessIdentity } from "../identity/provider/launch.mjs";
import { requireCheckpointedSource, requireOriginalIdentity } from "./checkpointed-source.mjs";
import {
  snapshotProviderCgroup,
  requireDrainedCgroup,
  createProviderCgroup,
  admitOwnedWrapper,
} from "../identity/provider/cgroup.mjs";
import { verifyPreservedBinary } from "../identity/provider/build-witness.mjs";
import { verifyLoadSources } from "./source-witness.mjs";
process.umask(0o077);
const run = "/var/tmp/rom-010-authentik-20261007/run/volume",
  failedPath = process.argv[2],
  buildPath = process.argv[3];
if (
  process.argv.length !== 4 ||
  !new RegExp(
    `^${run}/evidence/application-load-seed-[a-f0-9]{24}/result\\.json$`,
  ).test(failedPath) ||
  !new RegExp(
    `^${run}/evidence/application-load-build-[a-f0-9]{24}/build-identity\\.json$`,
  ).test(buildPath)
)
  throw Error("closed offline inspection inputs");
const failed = JSON.parse(readFileSync(failedPath)),
  build = JSON.parse(readFileSync(buildPath));
if (
  !new RegExp(`^${run}/private/application-recovery-[a-f0-9]{24}$`).test(
    failed.private_root,
  )
)
  throw Error("closed failed private directory");
if (
  !["sqlite", "redb"].includes(failed.adapter) ||
  failed.status !== "failed" ||
  !failed.source_fence_after
)
  throw Error("preserved failed source required");
requireDrainedCgroup(snapshotProviderCgroup(failed.group).cgroup_events);
if ([failed.process?.wrapper, failed.process?.target].some((identity) =>
  !identity || sameProcessIdentity(identity, readProcessIdentity(identity.pid))))
  throw Error("failed source process birth remains live or missing");
verifyLoadSources(build.sources);
verifyPreservedBinary(build.binary);
const nonce = randomBytes(12).toString("hex"),
  privateRoot = `${run}/private/application-recovery-${nonce}`,
  evidence = `${run}/evidence/application-load-inspection-${nonce}`;
for (const directory of [
  privateRoot,
  evidence,
  `${privateRoot}/destination`,
  `${privateRoot}/backup`,
])
  mkdirSync(directory, { mode: 0o700 });
const original = `${failed.private_root}/source/database`,
  destination = `${privateRoot}/destination/database`;
const absentSidecars = requireCheckpointedSource(original, failed.adapter);
function copy() {
  const input = openSync(original, constants.O_RDONLY | constants.O_NOFOLLOW),
    before = fstatSync(input),
    output = openSync(
      destination,
      constants.O_WRONLY |
        constants.O_CREAT |
        constants.O_EXCL |
        constants.O_NOFOLLOW,
      0o600,
    ),
    digest = createHash("sha256"),
    buffer = Buffer.alloc(65536);
  let bytes = 0;
  try {
    if (
      !before.isFile() ||
      before.nlink !== 1 ||
      before.size < 1 ||
      before.size > 256 * 1024 ** 2
    )
      throw Error("bounded offline original");
    for (;;) {
      const size = readSync(input, buffer, 0, buffer.length, null);
      if (!size) break;
      bytes += size;
      if (bytes > 256 * 1024 ** 2) throw Error("bounded offline copy");
      digest.update(buffer.subarray(0, size));
      let written = 0;
      while (written < size) {
        const amount = writeSync(output, buffer, written, size - written);
        if (amount <= 0) throw Error("offline copy stalled");
        written += amount;
      }
    }
    fsyncSync(output);
    const after = fstatSync(input);
    if (
      bytes !== before.size ||
      before.ino !== after.ino ||
      before.mtimeMs !== after.mtimeMs ||
      before.ctimeMs !== after.ctimeMs
    )
      throw Error("offline original changed");
    return {
      bytes,
      sha256: digest.digest("hex"),
      source_inode: before.ino,
      source_device: before.dev,
      copied_inode: fstatSync(output).ino,
    };
  } finally {
    closeSync(input);
    closeSync(output);
  }
}
const copied = copy(),
  configuration = JSON.parse(
    readFileSync(`${failed.private_root}/prepare.json`),
  );
Object.assign(configuration, {
  mode: "post-snapshot",
  directory: `${privateRoot}/destination`,
  source_directory: `${privateRoot}/destination`,
  backup_directory: `${privateRoot}/backup`,
  stop_file: `${privateRoot}/destination-stop`,
});
const path = `${privateRoot}/inspection.json`;
writeFileSync(path, JSON.stringify(configuration), { flag: "wx", mode: 0o600 });
const launcher = new OwnedLauncher(),
  group = createProviderCgroup();
let record = {
  adapter: failed.adapter,
  evidence,
  private_root: privateRoot,
  failed_source: failedPath,
  build_identity: buildPath,
  copy: copied,
  original_path: original,
  source_absent_sidecars_before: absentSidecars,
};
try {
  const child = launcher.launch(build.binary.path, [path], {
    timeoutMs: 30000,
    graceMs: 1000,
    drainMs: 1000,
    outputBytes: 65536,
    admitIdentity: (id) => admitOwnedWrapper(group, id),
  });
  let stdout = "",
    stderr = "";
  child.child.stdout.on("data", (bytes) => {
    stdout += bytes.toString("utf8");
  });
  child.child.stderr.on("data", (bytes) => {
    stderr += bytes.toString("utf8");
  });
  record.process = {
    wrapper: child.identity,
    target: await child.targetStarted,
    closed: await child.closed,
  };
  await child.physicalClose;
  writeFileSync(
    `${evidence}/process-output.json`,
    JSON.stringify({ stdout, stderr }),
    { flag: "wx", mode: 0o600 },
  );
  if (record.process.closed.exit_code !== 0)
    throw Error("actual copied inspector failed");
  record.inspection = JSON.parse(
    readFileSync(`${privateRoot}/destination/load-inspection.json`),
  );
  record.status = "passed";
} catch {
  record.status = "failed";
  process.exitCode = 1;
} finally {
  record.drain = await launcher.drain();
  record.group = snapshotProviderCgroup(group);
  requireDrainedCgroup(record.group.cgroup_events);
  const input = openSync(original, constants.O_RDONLY | constants.O_NOFOLLOW),
    digest = createHash("sha256"),
    buffer = Buffer.alloc(65536);
  let total = 0;
  try {
    requireOriginalIdentity(fstatSync(input), copied);
    for (;;) {
      const size = readSync(input, buffer, 0, buffer.length, null);
      if (!size) break;
      total += size;
      if (total > 256 * 1024 ** 2) throw Error("bounded original verification");
      digest.update(buffer.subarray(0, size));
    }
    requireOriginalIdentity(fstatSync(input), copied);
  } finally {
    closeSync(input);
  }
  const unchanged =
    total === copied.bytes && digest.digest("hex") === copied.sha256;
  record.original_hash_unchanged = unchanged;
  record.source_absent_sidecars_after = requireCheckpointedSource(original, failed.adapter);
  requireDrainedCgroup(snapshotProviderCgroup(failed.group).cgroup_events);
  try {
    verifyLoadSources(build.sources);
    verifyPreservedBinary(build.binary);
  } catch {
    record.status = "source-drift";
    process.exitCode = 1;
  }
  if (!unchanged) {
    record.status = "original-drift";
    process.exitCode = 1;
  }
  writeFileSync(
    `${evidence}/result.json`,
    JSON.stringify(record, null, 2) + "\n",
    { flag: "wx", mode: 0o600 },
  );
  console.log(
    JSON.stringify({
      result: `${evidence}/result.json`,
      status: record.status,
      inspection: record.inspection,
      original_hash_unchanged: unchanged,
    }),
  );
}
