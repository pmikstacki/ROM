import { requireOwnedListener } from "../identity/provider/network-admission.mjs";
import { requireWorkloadWindow } from "../identity/provider/window-profile.mjs";
import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  readlinkSync,
  lstatSync,
} from "node:fs";
import { randomBytes, createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { verifyPreservedBinary } from "../identity/provider/build-witness.mjs";
import { namespaceTools } from "../identity/provider/trust-namespace.mjs";
import { runOwned } from "./owned-command.mjs";
import { restoreObjects } from "./snapshot.mjs";
import { checkRecoveredHttp } from "./http-recovery.mjs";
const root = "/var/tmp/rom-010-authentik-20261007/run/volume";
process.umask(0o077);
const readyPath = process.argv[2],
  previous = JSON.parse(readFileSync(process.argv[3]));
if (previous.status !== "restore-checks-passed")
  throw Error("verified snapshot preparation required");
const build = JSON.parse(readFileSync(previous.build_identity));
verifyPreservedBinary(build.binary);
for (const source of build.sources)
  if (
    createHash("sha256").update(readFileSync(source.path)).digest("hex") !==
    source.sha256
  )
    throw Error("matching application source required");
const ready = JSON.parse(readFileSync(readyPath));
requireWorkloadWindow(ready.ready_admission, 210000);
requireOwnedListener(44390, ready.relay.target);
const base = JSON.parse(readFileSync(previous.private_root + "/prepare.json"));
const nonce = randomBytes(12).toString("hex"),
  directory = root + "/private/application-recovery-" + nonce,
  evidence = root + "/evidence/application-recovery-" + nonce;
for (const path of [
  directory,
  evidence,
  directory + "/destination",
  directory + "/empty",
])
  mkdirSync(path, { mode: 0o700 });
const configuration = {
  ...base,
  mode: "restore",
  directory: directory + "/destination",
  stop_file: directory + "/destination-stop",
};
writeFileSync(directory + "/prepare.json", JSON.stringify(configuration), {
  flag: "wx",
  mode: 0o600,
});
const record = {
  schema: 1,
  adapter: base.adapter,
  private_root: directory,
  evidence,
  build_identity: previous.build_identity,
  source_snapshot: previous.source_snapshot,
  last_post_snapshot_ack: previous.last_post_snapshot_ack,
  snapshot_preparation: process.argv[3],
  restore_started_unix_ms: Date.now(),
  status: "starting",
};
try {
  record.manifest = restoreObjects(
    base.backup_directory,
    directory + "/destination",
    build.binary.sha256,
  );
  const isolation = directory + "/restore-namespace.json";
  writeFileSync(
    isolation,
    JSON.stringify({
      source: base.source_directory,
      empty: directory + "/empty",
      private_root: directory,
      parent_namespace: readlinkSync("/proc/self/ns/mnt"),
      namespace_result: evidence + "/restore-namespace.json",
      binary: build.binary.path,
      host_configuration: directory + "/prepare.json",
      mode: "restore",
      process_result: evidence + "/restore-process.json",
    }),
    { flag: "wx", mode: 0o600 },
  );
  record.process = await runOwned(namespaceTools.unshare, [
    "--mount",
    "--propagation",
    "private",
    "--",
    process.execPath,
    fileURLToPath(new URL("./source-isolation-run.mjs", import.meta.url)),
    isolation,
  ]);
  if (record.process.closed.exit_code !== 0)
    throw Error("actual timed restore checks failed");
  record.database_attachment_work_checks = JSON.parse(
    readFileSync(directory + "/destination/restored.json"),
  );
  record.source_available_in_parent_after_recovery = lstatSync(
    base.source_directory + "/source-only-canary",
  ).isFile();
  record.status = "restore-checks-passed";
} catch (error) {
  record.status = "failed";
  record.failure = error.message;
  process.exitCode = 1;
}
writeFileSync(
  evidence + "/restore-result.json",
  JSON.stringify(record, null, 2),
  { flag: "wx", mode: 0o600 },
);
if (record.status === "restore-checks-passed")
  await checkRecoveredHttp(readyPath, evidence + "/restore-result.json");
else
  console.log(
    JSON.stringify({
      status: record.status,
      evidence,
      adapter: record.adapter,
    }),
  );
