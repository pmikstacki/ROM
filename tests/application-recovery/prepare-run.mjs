import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { randomBytes } from "node:crypto";
import { OwnedLauncher } from "../identity/provider/launch.mjs";
import {
  createProviderCgroup,
  admitOwnedWrapper,
  snapshotProviderCgroup,
  requireDrainedCgroup,
} from "../identity/provider/cgroup.mjs";
import { verifyPreservedBinary } from "../identity/provider/build-witness.mjs";
process.umask(0o077);
const root = "/var/tmp/rom-010-authentik-20261007/run/volume",
  build = JSON.parse(readFileSync(process.argv[2])),
  adapter = process.argv[3];
if (!["sqlite", "redb"].includes(adapter))
  throw Error("closed recovery adapter");
verifyPreservedBinary(build.binary);
const nonce = randomBytes(12).toString("hex"),
  privateRoot = root + "/private/application-recovery-" + nonce,
  evidence = root + "/evidence/application-recovery-" + nonce;
for (const p of [
  privateRoot,
  evidence,
  privateRoot + "/source",
  privateRoot + "/backup",
  privateRoot + "/destination",
  privateRoot + "/empty",
])
  mkdirSync(p, { mode: 0o700 });
const synthetic = root + "/private/authentik-96b706d518977e471f458dfff7b9503a",
  client = JSON.parse(readFileSync(synthetic + "/oidc-client.json")),
  flow = JSON.parse(
    readFileSync(synthetic + "/original-code-flow-attempt16.json"),
  ),
  subject = JSON.parse(
    Buffer.from(flow.id_token.split(".")[1], "base64url"),
  ).sub;
const configuration = {
  mode: "prepare",
  adapter,
  directory: privateRoot + "/source",
  source_directory: privateRoot + "/source",
  backup_directory: privateRoot + "/backup",
  issuer: "https://127.0.0.1:44392/application/o/rom-synthetic-identity/",
  client_id: client.client_id,
  client_secret: client.client_secret,
  verified_synthetic_subject: subject,
  stop_file: privateRoot + "/source-stop",
};
const path = privateRoot + "/prepare.json";
writeFileSync(path, JSON.stringify(configuration), { flag: "wx", mode: 0o600 });
const launcher = new OwnedLauncher(),
  group = createProviderCgroup();
let record = {
  adapter,
  private_root: privateRoot,
  evidence,
  build_identity: process.argv[2],
};
try {
  const child = launcher.launch(build.binary.path, [path], {
    timeoutMs: 60000,
    graceMs: 1000,
    drainMs: 1000,
    outputBytes: 65536,
    admitIdentity: (id) => admitOwnedWrapper(group, id),
  });
  child.child.stdout.on("data", () => {});
  child.child.stderr.on("data", (v) => process.stderr.write(v));
  record.process = {
    wrapper: child.identity,
    target: await child.targetStarted,
    closed: await child.closed,
  };
  await child.physicalClose;
  process.exitCode = record.process.closed.exit_code === 0 ? 0 : 1;
} finally {
  record.drain = await launcher.drain();
  record.group = snapshotProviderCgroup(group);
  requireDrainedCgroup(record.group.cgroup_events);
  writeFileSync(
    evidence + "/preparation-result.json",
    JSON.stringify(record, null, 2),
    { flag: "wx", mode: 0o600 },
  );
  console.log(
    JSON.stringify({
      result: evidence + "/preparation-result.json",
      status: record.process?.closed.exit_code === 0 ? "passed" : "failed",
    }),
  );
}
