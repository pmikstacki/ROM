import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, symlinkSync, rmSync, statSync, renameSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { requireCheckpointedSource, requireOriginalIdentity } from "./checkpointed-source.mjs";

test("identical content in a replaced inode does not establish preserved original identity", () => {
  const directory = mkdtempSync(join(tmpdir(), "rom-load-inode-"));
  const database = join(directory, "database");
  try {
    writeFileSync(database, "same bytes");
    const prior = statSync(database);
    const proof = { source_inode: prior.ino, source_device: prior.dev, bytes: prior.size };
    assert.doesNotThrow(() => requireOriginalIdentity(prior, proof));
    writeFileSync(join(directory, "replacement"), "same bytes");
    renameSync(join(directory, "replacement"), database);
    assert.throws(() => requireOriginalIdentity(statSync(database), proof), /original identity/);
  } finally {
    rmSync(directory, { recursive: true });
  }
});

test("SQLite detached-copy admission refuses every existing sidecar, including dangling links", () => {
  const directory = mkdtempSync(join(tmpdir(), "rom-load-copy-"));
  const database = join(directory, "database");
  try {
    writeFileSync(database, "preserved original");
    assert.doesNotThrow(() => requireCheckpointedSource(database, "sqlite"));
    for (const suffix of ["-wal", "-shm", "-journal"]) {
      writeFileSync(database + suffix, "uncheckpointed");
      assert.throws(() => requireCheckpointedSource(database, "sqlite"), /sidecar/);
      rmSync(database + suffix);
      symlinkSync(join(directory, "absent-target"), database + suffix);
      assert.throws(() => requireCheckpointedSource(database, "sqlite"), /sidecar/);
      rmSync(database + suffix);
    }
    assert.throws(() => requireCheckpointedSource(database, "unknown"));
  } finally {
    rmSync(directory, { recursive: true });
  }
});
