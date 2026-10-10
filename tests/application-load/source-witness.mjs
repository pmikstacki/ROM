import {
  captureSourceTree,
  requireSourceFence,
} from "../identity/provider/build-witness.mjs";
import { createHash } from "node:crypto";
import { readFileSync, readdirSync, lstatSync, realpathSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { captureWorkParserSources, readParserSource } from "./work-parser.mjs";
const root = realpathSync(fileURLToPath(new URL("../../", import.meta.url)));
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
export function captureLoadHarnessSources() {
  const directory = `${root}/tests/application-load`;
  const paths = readdirSync(directory)
    .filter((name) => name.endsWith(".mjs"))
    .map((name) => `${directory}/${name}`)
    .sort();
  if (paths.length > 256)
    throw Error("load harness source count exceeds limit");
  let bytes = 0;
  for (const path of paths) {
    const stat = lstatSync(path);
    bytes += stat.size;
    if (
      !stat.isFile() ||
      stat.isSymbolicLink() ||
      stat.size > 65536 ||
      bytes > 4194304
    )
      throw Error("load harness source bytes exceed limit");
  }
  const sources = paths.map((path) => ({
    path,
    sha256: createHash("sha256")
      .update(readParserSource(path).raw)
      .digest("hex"),
  }));
  return sources
    .concat(
      captureWorkParserSources().map(({ path, sha256 }) => ({ path, sha256 })),
    )
    .sort((a, b) => a.path.localeCompare(b.path));
}
export function captureLoadSources() {
  const directories = crates
    .map((name) => `${root}/crates/${name}/src`)
    .concat([
      `${root}/demo/src`,
      `${root}/tests/application-recovery/host/src`,
      `${root}/tests/application-load/host/src`,
    ]);
  const files = crates
    .map((name) => `${root}/crates/${name}/Cargo.toml`)
    .concat([
      `${root}/demo/Cargo.toml`,
      `${root}/Cargo.toml`,
      `${root}/Cargo.lock`,
      `${root}/tests/application-load/host/Cargo.toml`,
      `${root}/tests/application-load/host/Cargo.lock`,
    ]);
  return directories
    .flatMap(captureSourceTree)
    .concat(
      files.map((path) => ({
        path,
        sha256: createHash("sha256").update(readFileSync(path)).digest("hex"),
      })),
    )
    .concat(captureLoadHarnessSources())
    .sort((a, b) => a.path.localeCompare(b.path));
}
export function verifyLoadSources(sources) {
  requireSourceFence(sources, captureLoadSources());
}
