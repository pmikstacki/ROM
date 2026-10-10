import test from "node:test";
import assert from "node:assert/strict";
import { validateManifest } from "./manifest.mjs";
const valid = () => ({
  schema: 1,
  adapter: "sqlite",
  archive: { name: "database.archive", bytes: 500, sha256: "a".repeat(64) },
  objects: Array.from({ length: 20 }, (_, i) => ({
    name: i.toString(16).padStart(64, "0"),
    bytes: 12,
    sha256: "c".repeat(64),
  })),
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
  binary_sha256: "d".repeat(64),
});
test("recovery inventory accepts only the supported bounded reference profile", () => {
  assert.equal(validateManifest(valid()).adapter, "sqlite");
  for (const key of ["identity", "credential", "object_store"]) {
    const v = valid();
    v.references[key] = "";
    assert.throws(() => validateManifest(v));
  }
});
test("archive and object bytes are finite and names cannot escape the snapshot", () => {
  for (const mutate of [
    (v) => (v.archive.bytes = 128 * 1024 ** 2),
    (v) => (v.objects[0].name = "../source"),
    (v) => v.objects.push(v.objects[0]),
    (v) => (v.objects[0].sha256 = "wrong"),
    (v) => (v.snapshot.post_snapshot_mutations = 21),
  ]) {
    const v = valid();
    mutate(v);
    assert.throws(() => validateManifest(v));
  }
});
test("secrets and unknown schema attributes are rejected, not silently copied", () => {
  const v = valid();
  v.references.client_secret = "do-not-copy";
  assert.throws(() => validateManifest(v));
  const x = valid();
  x.adapter = "other";
  assert.throws(() => validateManifest(x));
});
test("reference property order does not change the recovery contract", () => {
  const value = valid();
  value.references = {
    credential: "synthetic-oidc-client",
    identity: "authentik",
    object_store: "attachments",
  };
  assert.equal(validateManifest(value).adapter, "sqlite");
});
