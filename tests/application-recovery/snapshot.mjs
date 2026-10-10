import {
  openSync,
  closeSync,
  fstatSync,
  lstatSync,
  readSync,
  writeSync,
  fsyncSync,
  readdirSync,
  writeFileSync,
  readFileSync,
  existsSync,
  mkdirSync,
  realpathSync,
  constants,
} from "node:fs";
import { createHash } from "node:crypto";
import { validateManifest } from "./manifest.mjs";
function ownedDirectory(path) {
  const value = lstatSync(path);
  if (
    realpathSync(path) !== path ||
    !value.isDirectory() ||
    value.uid !== process.getuid() ||
    (value.mode & 0o777) !== 0o700
  )
    throw Error("exclusive private snapshot directory required");
}
export function copyObject(source, destination, limit, expected) {
  const input = openSync(source, constants.O_RDONLY | constants.O_NOFOLLOW),
    hash = createHash("sha256"),
    buffer = Buffer.alloc(65536);
  let output,
    total = 0;
  try {
    const before = fstatSync(input);
    if (
      !before.isFile() ||
      before.nlink !== 1 ||
      before.size <= 0 ||
      before.size > limit ||
      before.uid !== process.getuid()
    )
      throw Error("bounded regular snapshot source required");
    if (destination) output = openSync(destination, "wx", 0o600);
    for (;;) {
      const count = readSync(input, buffer, 0, buffer.length, null);
      if (!count) break;
      if ((total += count) > limit) throw Error("snapshot byte bound");
      hash.update(buffer.subarray(0, count));
      if (output !== undefined) {
        let offset = 0;
        while (offset < count) {
          const done = writeSync(output, buffer, offset, count - offset);
          if (done <= 0) throw Error("snapshot copy stalled");
          offset += done;
        }
      }
    }
    if (output !== undefined) fsyncSync(output);
    const after = fstatSync(input),
      sha256 = hash.digest("hex");
    if (
      before.ino !== after.ino ||
      before.dev !== after.dev ||
      before.mtimeMs !== after.mtimeMs ||
      before.ctimeMs !== after.ctimeMs ||
      before.size !== after.size ||
      total !== before.size
    )
      throw Error("snapshot source changed");
    if (expected && (expected.bytes !== total || expected.sha256 !== sha256))
      throw Error("snapshot digest mismatch");
    return { bytes: total, sha256 };
  } finally {
    closeSync(input);
    if (output !== undefined) closeSync(output);
  }
}
export function snapshotObjects(source, backup, adapter, binarySha) {
  ownedDirectory(source);
  ownedDirectory(backup);
  const paths = readdirSync(source);
  if (paths.length !== 20 || paths.some((v) => !/^[a-f0-9]{64}$/.test(v)))
    throw Error("complete attachment inventory required");
  const folder = backup + "/objects";
  mkdirSync(folder, { mode: 0o700 });
  const objects = paths
    .sort()
    .map((name) => ({
      name,
      ...copyObject(source + "/" + name, folder + "/" + name, 65536),
    }));
  const value = {
    schema: 1,
    adapter,
    archive: {
      name: "database.archive",
      ...copyObject(backup + "/database.archive", null, 128 * 1024 ** 2),
    },
    objects,
    references: {
      object_store: "attachments",
      identity: "authentik",
      credential: "synthetic-oidc-client",
    },
    snapshot: {
      resources: 1000,
      tombstones: 100,
      attachments: 20,
      post_snapshot_mutations: 20,
    },
    binary_sha256: binarySha,
  };
  validateManifest(value);
  writeFileSync(backup + "/manifest.json", JSON.stringify(value, null, 2), {
    flag: "wx",
    mode: 0o600,
  });
  return value;
}
export function restoreObjects(backup, destination, binarySha) {
  ownedDirectory(backup);
  ownedDirectory(destination);
  const path = backup + "/manifest.json";
  if (lstatSync(path).size > 65536) throw Error("manifest bound");
  const manifest = validateManifest(JSON.parse(readFileSync(path)));
  if (manifest.binary_sha256 !== binarySha)
    throw Error("matching application binary required");
  if (
    existsSync(destination + "/objects") ||
    existsSync(destination + "/database")
  )
    throw Error("fresh recovery destination required");
  copyObject(
    backup + "/database.archive",
    null,
    128 * 1024 ** 2,
    manifest.archive,
  );
  for (const object of manifest.objects)
    copyObject(backup + "/objects/" + object.name, null, 65536, object);
  mkdirSync(destination + "/objects", { mode: 0o700 });
  for (const object of manifest.objects)
    copyObject(
      backup + "/objects/" + object.name,
      destination + "/objects/" + object.name,
      65536,
      object,
    );
  return manifest;
}
