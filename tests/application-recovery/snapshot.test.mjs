import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, chmodSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { copyObject } from "./snapshot.mjs";
test("snapshot copy checks exact bytes and digest without overwriting destination", () => {
  const root = mkdtempSync(tmpdir() + "/rom-application-copy-");
  chmodSync(root, 0o700);
  writeFileSync(root + "/source", "private fixture bytes", { mode: 0o600 });
  const identity = copyObject(root + "/source", root + "/copy", 64);
  assert.equal(readFileSync(root + "/copy", "utf8"), "private fixture bytes");
  assert.throws(() => copyObject(root + "/source", root + "/copy", 64));
  assert.throws(() =>
    copyObject(root + "/source", null, 64, {
      ...identity,
      sha256: "0".repeat(64),
    }),
  );
  assert.throws(() => copyObject(root + "/source", null, 1));
});
import { mkdirSync, existsSync } from "node:fs";
import { snapshotObjects, restoreObjects } from "./snapshot.mjs";
function fixture() {
  const root = mkdtempSync(tmpdir() + "/rom-application-inventory-");
  chmodSync(root, 0o700);
  for (const name of ["source", "backup", "destination"])
    mkdirSync(root + "/" + name, { mode: 0o700 });
  for (let i = 0; i < 20; i++)
    writeFileSync(
      root + "/source/" + i.toString(16).padStart(64, "0"),
      `actual object ${i}`,
      { mode: 0o600 },
    );
  writeFileSync(
    root + "/backup/database.archive",
    "synthetic archive fixture",
    { mode: 0o600 },
  );
  const binary = "a".repeat(64);
  snapshotObjects(root + "/source", root + "/backup", "sqlite", binary);
  return { root, binary };
}
test("fresh object restore uses only digested backup bytes and rejects repeat publication", () => {
  const { root, binary } = fixture();
  restoreObjects(root + "/backup", root + "/destination", binary);
  assert.equal(
    readFileSync(root + "/destination/objects/" + "0".repeat(64), "utf8"),
    "actual object 0",
  );
  assert.throws(() =>
    restoreObjects(root + "/backup", root + "/destination", binary),
  );
});
test("changed object or unresolved reference prevents any restored object publication", () => {
  for (const mutation of ["bytes", "reference"]) {
    const { root, binary } = fixture();
    if (mutation === "bytes")
      writeFileSync(root + "/backup/objects/" + "0".repeat(64), "changed");
    else {
      const path = root + "/backup/manifest.json",
        value = JSON.parse(readFileSync(path));
      value.references.credential = "";
      writeFileSync(path, JSON.stringify(value));
    }
    assert.throws(() =>
      restoreObjects(root + "/backup", root + "/destination", binary),
    );
    assert.equal(existsSync(root + "/destination/objects"), false);
  }
});
