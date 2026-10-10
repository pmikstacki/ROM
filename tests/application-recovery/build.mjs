import {
  mkdirSync,
  readFileSync,
  writeFileSync,
  lstatSync,
  statfsSync,
} from "node:fs";
import { createHash, randomBytes } from "node:crypto";
import {
  captureSourceTree,
  requireSourceFence,
  preserveBinary,
} from "../identity/provider/build-witness.mjs";
const root = "/root/ROM",
  run = "/var/tmp/rom-010-authentik-20261007/run/volume";
const crates = [
  "rom",
  "rom-fields",
  "rom-sqlite",
  "rom-redb",
  "rom-http",
  "rom-config",
  "rom-identity",
  "rom-blob",
  "rom-blob-object-store",
  "rom-backup",
  "rom-auth",
  "rom-derive",
  "rom-studio-host",
];
const hash = (path) =>
  createHash("sha256").update(readFileSync(path)).digest("hex");
function capture() {
  const directories = crates
    .map((name) => `${root}/crates/${name}/src`)
    .concat([
      `${root}/demo/src`,
      `${root}/tests/application-recovery/host/src`,
    ]);
  const files = crates
    .map((name) => `${root}/crates/${name}/Cargo.toml`)
    .concat([
      `${root}/demo/Cargo.toml`,
      `${root}/Cargo.toml`,
      `${root}/Cargo.lock`,
      `${root}/tests/application-recovery/host/Cargo.toml`,
      `${root}/tests/application-recovery/host/Cargo.lock`,
      `${root}/tests/application-recovery/build.mjs`,
    ]);
  return directories
    .flatMap(captureSourceTree)
    .concat(files.map((path) => ({ path, sha256: hash(path) })))
    .sort((a, b) => a.path.localeCompare(b.path));
}
const command =
  "CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 timeout --signal=TERM --kill-after=5s 120s cargo build --manifest-path tests/application-recovery/host/Cargo.toml --locked --offline --target-dir /workspace/ROM/target/application-recovery --message-format=json";
if (process.argv[2] === "prepare") {
  if (
    Number(statfsSync(run).bavail) * Number(statfsSync(run).bsize) <
    1024 ** 3
  )
    throw Error("private run headroom");
  const nonce = randomBytes(12).toString("hex"),
    evidence = `${run}/evidence/application-recovery-build-${nonce}`,
    privateRoot = `${run}/private/application-recovery-${nonce}`;
  mkdirSync(evidence, { mode: 0o700 });
  mkdirSync(privateRoot, { mode: 0o700 });
  const value = {
    evidence,
    private_root: privateRoot,
    command,
    sources: capture(),
    shared_target_planned_bytes: 8 * 1024 ** 3,
    shared_target_hard_quota: false,
    private_snapshot_bytes: 256 * 1024 ** 2,
  };
  writeFileSync(
    `${evidence}/preparation.json`,
    JSON.stringify(value, null, 2),
    { flag: "wx", mode: 0o600 },
  );
  console.log(
    JSON.stringify({
      preparation: `${evidence}/preparation.json`,
      command,
      source_count: value.sources.length,
    }),
  );
} else if (process.argv[2] === "finish") {
  const path = process.argv[3];
  if (
    !new RegExp(
      `^${run}/evidence/application-recovery-build-[a-f0-9]{24}/preparation\\.json$`,
    ).test(path) ||
    lstatSync(path).size > 1048576
  )
    throw Error("owned preparation");
  const p = JSON.parse(readFileSync(path));
  requireSourceFence(p.sources, capture());
  const events = readFileSync(`${p.evidence}/build.jsonl`, "utf8")
    .split("\n")
    .filter((line) => line.startsWith("{"))
    .map((line) => JSON.parse(line));
  if (
    !events.some((v) => v.reason === "build-finished" && v.success) ||
    !events.some(
      (v) =>
        v.reason === "compiler-artifact" &&
        v.target?.name === "rom-application-recovery-authoring" &&
        v.executable ===
          "/workspace/ROM/target/application-recovery/debug/rom-application-recovery-authoring",
    )
  )
    throw Error("original compiler artifact required");
  const binary = preserveBinary(
    `${root}/target/application-recovery/debug/rom-application-recovery-authoring`,
    `${p.private_root}/host`,
  );
  requireSourceFence(p.sources, capture());
  const identity = {
    ...p,
    binary,
    scope: "current-source-authoring",
    artifact_admission: false,
  };
  writeFileSync(
    `${p.evidence}/build-identity.json`,
    JSON.stringify(identity, null, 2),
    { flag: "wx", mode: 0o600 },
  );
  console.log(
    JSON.stringify({
      identity: `${p.evidence}/build-identity.json`,
      binary: binary.path,
      sha256: binary.sha256,
      bytes: binary.bytes,
    }),
  );
} else throw Error("prepare or finish");
