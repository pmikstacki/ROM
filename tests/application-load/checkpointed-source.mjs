// Admission for copying a stopped database. Never open the original through SQLite.
import { lstatSync } from "node:fs";
export function requireOriginalIdentity(actual, proof) {
  if (!actual.isFile() || actual.nlink !== 1 || actual.ino !== proof.source_inode ||
      actual.dev !== proof.source_device || actual.size !== proof.bytes)
    throw Error("preserved original identity changed");
}
export function requireCheckpointedSource(database, adapter) {
  if (!["sqlite", "redb"].includes(adapter) || typeof database !== "string")
    throw Error("closed detached-copy adapter");
  if (adapter === "redb") return [];
  const absent = [];
  for (const suffix of ["-wal", "-shm", "-journal"]) {
    try {
      lstatSync(database + suffix);
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
      absent.push(suffix);
      continue;
    }
    throw Error("SQLite source sidecar requires preserved checkpoint evidence");
  }
  return absent;
}
