import {
  mkdirSync,
  readFileSync,
  writeFileSync,
  lstatSync,
  statfsSync,
} from "node:fs";
import { randomBytes } from "node:crypto";
import { captureLoadSources, verifyLoadSources } from "./source-witness.mjs";
import { loadBuildPlan } from "./build-plan.mjs";
import { preserveBinary } from "../identity/provider/build-witness.mjs";
const run = "/var/tmp/rom-010-authentik-20261007/run/volume";
function save(path, value) {
  writeFileSync(path, JSON.stringify(value, null, 2) + "\n", {
    flag: "wx",
    mode: 0o600,
  });
}
if (
  process.argv[2] === "prepare" &&
  (process.argv.length === 3 ||
    (process.argv.length === 4 &&
      ["--release", "--stage-diagnostic"].includes(process.argv[3])))
) {
  const plan = loadBuildPlan(
    process.argv[3] === "--stage-diagnostic"
      ? "stage-diagnostic"
      : process.argv[3] === "--release"
        ? "release"
        : "dev",
  );
  const command = plan.command;
  const disk = statfsSync(run);
  if (Number(disk.bavail) * Number(disk.bsize) < 1024 ** 3)
    throw Error("private finite run headroom");
  const nonce = randomBytes(12).toString("hex"),
    evidence = `${run}/evidence/application-load-build-${nonce}`,
    privateRoot = `${run}/private/application-load-build-${nonce}`;
  mkdirSync(evidence, { mode: 0o700 });
  mkdirSync(privateRoot, { mode: 0o700 });
  const value = {
    evidence,
    private_root: privateRoot,
    command,
    plan,
    sources: captureLoadSources(),
    shared_target_planned_bytes: 8 * 1024 ** 3,
    shared_target_hard_quota: false,
    binary_preservation_bytes: 256 * 1024 ** 2,
    private_run_envelope_bytes: 32 * 1024 ** 3,
  };
  save(`${evidence}/preparation.json`, value);
  console.log(
    JSON.stringify({
      preparation: `${evidence}/preparation.json`,
      command,
      source_count: value.sources.length,
    }),
  );
} else if (process.argv[2] === "finish" && process.argv.length === 4) {
  const path = process.argv[3];
  if (
    !new RegExp(
      `^${run}/evidence/application-load-build-[a-f0-9]{24}/preparation\\.json$`,
    ).test(path) ||
    lstatSync(path).isSymbolicLink() ||
    lstatSync(path).size > 1048576
  )
    throw Error("owned load preparation");
  const value = JSON.parse(readFileSync(path));
  verifyLoadSources(value.sources);
  const log = `${value.evidence}/build.jsonl`;
  if (lstatSync(log).isSymbolicLink() || lstatSync(log).size > 16 * 1024 ** 2)
    throw Error("finite compiler output");
  const events = readFileSync(log, "utf8")
    .split("\n")
    .filter((line) => line.startsWith("{"))
    .map((line) => JSON.parse(line));
  if (
    !events.some(
      (event) => event.reason === "build-finished" && event.success,
    ) ||
    !events.some(
      (event) =>
        event.reason === "compiler-artifact" &&
        event.target?.name === "rom-application-load-authoring" &&
        event.executable === value.plan.executable,
    )
  )
    throw Error("actual load compiler artifact required");
  const binary = preserveBinary(
    value.plan.executable.replace("/workspace/ROM", "/root/ROM"),
    `${value.private_root}/host`,
  );
  verifyLoadSources(value.sources);
  save(`${value.evidence}/build-identity.json`, {
    ...value,
    binary,
    scope: value.plan.diagnostic
      ? "storage-stage-diagnostic-only"
      : "current-source-exploratory-load",
    production_throughput_admission: false,
    artifact_admission: false,
  });
  console.log(
    JSON.stringify({
      identity: `${value.evidence}/build-identity.json`,
      binary: binary.path,
      sha256: binary.sha256,
      bytes: binary.bytes,
    }),
  );
} else throw Error("closed prepare or finish arguments");
