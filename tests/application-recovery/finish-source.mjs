import {
  readFileSync,
  writeFileSync,
  readlinkSync,
  lstatSync,
  statfsSync,
} from "node:fs";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";
import { verifyPreservedBinary } from "../identity/provider/build-witness.mjs";
import { namespaceTools } from "../identity/provider/trust-namespace.mjs";
import { snapshotObjects, restoreObjects } from "./snapshot.mjs";
import { runOwned } from "./owned-command.mjs";
process.umask(0o077);
const prepared = JSON.parse(readFileSync(process.argv[2])),
  build = JSON.parse(readFileSync(prepared.build_identity)),
  directory = prepared.private_root,
  evidence = prepared.evidence,
  base = JSON.parse(readFileSync(directory + "/prepare.json"));
verifyPreservedBinary(build.binary);
for (const item of build.sources)
  if (
    createHash("sha256").update(readFileSync(item.path)).digest("hex") !==
    item.sha256
  )
    throw Error("current source fence required");
if (prepared.process.closed.exit_code !== 0)
  throw Error("successful source preparation required");
const record = {
  schema: 1,
  adapter: base.adapter,
  build_identity: prepared.build_identity,
  private_root: directory,
  status: "starting",
  phases: [],
  source_available_before_recovery: lstatSync(
    directory + "/source/source-only-canary",
  ).isFile(),
};
async function phase(mode, configuration, isolated = false) {
  const path = directory + "/" + mode + ".json";
  writeFileSync(path, JSON.stringify({ ...base, ...configuration, mode }), {
    flag: "wx",
    mode: 0o600,
  });
  let executable = build.binary.path,
    args = [path];
  if (isolated) {
    const isolation = directory + "/" + mode + "-namespace.json";
    writeFileSync(
      isolation,
      JSON.stringify({
        source: base.source_directory,
        empty: directory + "/empty",
        private_root: directory,
        parent_namespace: readlinkSync("/proc/self/ns/mnt"),
        namespace_result: evidence + "/" + mode + "-namespace.json",
        binary: build.binary.path,
        host_configuration: path,
        mode,
        process_result: evidence + "/" + mode + "-process.json",
      }),
      { flag: "wx", mode: 0o600 },
    );
    executable = namespaceTools.unshare;
    args = [
      "--mount",
      "--propagation",
      "private",
      "--",
      process.execPath,
      fileURLToPath(new URL("./source-isolation-run.mjs", import.meta.url)),
      isolation,
    ];
  }
  const result = await runOwned(executable, args);
  record.phases.push({ mode, ...result });
  if (result.closed.exit_code !== 0)
    throw Error("actual recovery phase failed: " + mode);
}
try {
  record.manifest = snapshotObjects(
    directory + "/source/objects",
    directory + "/backup",
    base.adapter,
    build.binary.sha256,
  );
  record.snapshot_time_ms = Date.now();
  record.source_snapshot = JSON.parse(
    readFileSync(directory + "/source/prepared.json"),
  );
  await phase("post-snapshot", { directory: directory + "/source" });
  record.last_post_snapshot_ack = JSON.parse(
    readFileSync(directory + "/source/post-snapshot.json"),
  );
  const begin = Date.now();
  record.restore_started_unix_ms = begin;
  restoreObjects(
    directory + "/backup",
    directory + "/destination",
    build.binary.sha256,
  );
  await phase(
    "restore",
    {
      directory: directory + "/destination",
      stop_file: directory + "/destination-stop",
    },
    true,
  );
  record.database_attachment_work_checks = JSON.parse(
    readFileSync(directory + "/destination/restored.json"),
  );
  record.restore_checks_elapsed_ms = Date.now() - begin;
  record.source_available_in_parent_after_recovery = lstatSync(
    directory + "/source/source-only-canary",
  ).isFile();
  record.status = "restore-checks-passed";
  record.whole_application_rto_pending_identity = true;
} catch (error) {
  record.status = "failed";
  record.failure = error.message;
  process.exitCode = 1;
} finally {
  record.private_free_bytes =
    Number(statfsSync(directory).bavail) * Number(statfsSync(directory).bsize);
  writeFileSync(
    evidence + "/restore-result.json",
    JSON.stringify(record, null, 2),
    { flag: "wx", mode: 0o600 },
  );
  console.log(
    JSON.stringify({ status: record.status, evidence, adapter: base.adapter }),
  );
}
