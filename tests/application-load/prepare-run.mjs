import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  lstatSync,
  statfsSync,
} from "node:fs";
import { randomBytes, createHash } from "node:crypto";
import { OwnedLauncher } from "../identity/provider/launch.mjs";
import {
  createProviderCgroup,
  admitOwnedWrapper,
  snapshotProviderCgroup,
  requireDrainedCgroup,
} from "../identity/provider/cgroup.mjs";
import { verifyPreservedBinary } from "../identity/provider/build-witness.mjs";
import { verifyLoadSources } from "./source-witness.mjs";
import { exploratoryProfile, requireSeedProof } from "./profile.mjs";
process.umask(0o077);
const run = "/var/tmp/rom-010-authentik-20261007/run/volume",
  path = process.argv[2],
  adapter = process.argv[3];
if (
  process.argv.length !== 4 ||
  !["sqlite", "redb"].includes(adapter) ||
  !new RegExp(
    `^${run}/evidence/application-load-build-[a-f0-9]{24}/build-identity\\.json$`,
  ).test(path)
)
  throw Error("closed load preparation");
const build = JSON.parse(readFileSync(path));
verifyPreservedBinary(build.binary);
verifyLoadSources(build.sources);
const disk = statfsSync(run);
if (Number(disk.bavail) * Number(disk.bsize) < 1024 ** 3)
  throw Error("finite private headroom");
const nonce = randomBytes(12).toString("hex"),
  privateRoot = `${run}/private/application-recovery-${nonce}`,
  evidence = `${run}/evidence/application-load-seed-${nonce}`;
for (const directory of [
  privateRoot,
  evidence,
  `${privateRoot}/source`,
  `${privateRoot}/backup`,
])
  mkdirSync(directory, { mode: 0o700 });
const synthetic = `${run}/private/authentik-96b706d518977e471f458dfff7b9503a`,
  flowPath = `${synthetic}/original-code-flow-attempt16.json`;
if (lstatSync(flowPath).size > 65536)
  throw Error("bounded original identity flow");
const flowBytes = readFileSync(flowPath),
  verified = JSON.parse(
    readFileSync(
      `${run}/evidence/sdk-authoring-20261007/actual-sdk-fresh-current-after-key-fix.json`,
    ),
  );
if (
  verified.private_input_identity.sha256 !==
    createHash("sha256").update(flowBytes).digest("hex") ||
  JSON.parse(verified.stdout).accepted !== true
)
  throw Error("actual original provider mapping required");
const subject = JSON.parse(
    Buffer.from(JSON.parse(flowBytes).id_token.split(".")[1], "base64url"),
  ).sub,
  client = JSON.parse(readFileSync(`${synthetic}/oidc-client.json`));
if (typeof subject !== "string" || subject.length > 512)
  throw Error("bounded synthetic subject");
const configuration = {
  mode: "prepare",
  adapter,
  directory: `${privateRoot}/source`,
  source_directory: `${privateRoot}/source`,
  backup_directory: `${privateRoot}/backup`,
  issuer: "https://127.0.0.1:44392/application/o/rom-synthetic-identity/",
  client_id: client.client_id,
  client_secret: client.client_secret,
  verified_synthetic_subject: subject,
  stop_file: `${privateRoot}/source-stop`,
};
const configurationPath = `${privateRoot}/prepare.json`;
writeFileSync(configurationPath, JSON.stringify(configuration), {
  flag: "wx",
  mode: 0o600,
});
const group = createProviderCgroup(),
  launcher = new OwnedLauncher();
let record = {
  adapter,
  private_root: privateRoot,
  evidence,
  build_identity: path,
  profile: exploratoryProfile(),
  source_fence_before: true,
  directory_planned_bytes: 256 * 1024 ** 2,
  directory_hard_quota: false,
};
if (build.plan.diagnostic) {
  record.diagnostic = {
    kind: "fixed-storage-stage-wall-time",
    acceptance: false,
    observer_overhead: "unmeasured",
    unchanged_profile: true,
  };
}
try {
  const child = launcher.launch(build.binary.path, [configurationPath], {
    timeoutMs: 130000,
    graceMs: 1000,
    drainMs: 1000,
    outputBytes: 65536,
    admitIdentity: (id) => admitOwnedWrapper(group, id),
  });
  const output = { stdout: "", stderr: "" };
  child.child.stdout.on("data", (value) => {
    output.stdout += value.toString("utf8");
  });
  child.child.stderr.on("data", (value) => {
    output.stderr += value.toString("utf8");
  });
  record.process = {
    wrapper: child.identity,
    target: await child.targetStarted,
    closed: await child.closed,
  };
  await child.physicalClose;
  writeFileSync(`${evidence}/process-output.json`, JSON.stringify(output), {
    flag: "wx",
    mode: 0o600,
  });
  if (record.process.closed.exit_code !== 0 || !record.process.closed.drained)
    throw Error("actual seed process failed");
  const proof = JSON.parse(
      readFileSync(`${privateRoot}/source/load-seed-proof.json`),
    ),
    { elapsed_ms, ...admission } = proof;
  requireSeedProof(record.profile, admission);
  if (!Number.isSafeInteger(elapsed_ms) || elapsed_ms > 120000)
    throw Error("actual seed deadline proof");
  record.seed = proof;
  record.status = build.plan.diagnostic ? "diagnostic-complete" : "passed";
} catch {
  record.status = "failed";
  process.exitCode = 1;
} finally {
  record.drain = await launcher.drain();
  record.group = snapshotProviderCgroup(group);
  requireDrainedCgroup(record.group.cgroup_events);
  try {
    verifyLoadSources(build.sources);
    verifyPreservedBinary(build.binary);
    record.source_fence_after = true;
  } catch {
    record.source_fence_after = false;
    record.status = "source-drift";
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
      adapter,
      seed: record.seed,
    }),
  );
}
