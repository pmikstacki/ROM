// Host-owned recovery inventory. References are names; this record contains no credentials.
const maxBytes = 128 * 1024 ** 2;
const exact = (value, keys) =>
  value &&
  typeof value === "object" &&
  !Array.isArray(value) &&
  JSON.stringify(Object.keys(value).sort()) ===
    JSON.stringify([...keys].sort());
function object(value, archive = false) {
  if (
    !exact(value, ["name", "bytes", "sha256"]) ||
    !(archive
      ? value.name === "database.archive"
      : /^[a-f0-9]{64}$/.test(value.name)) ||
    !Number.isSafeInteger(value.bytes) ||
    value.bytes <= 0 ||
    value.bytes > maxBytes ||
    !/^[a-f0-9]{64}$/.test(value.sha256)
  )
    throw Error("invalid bounded snapshot object");
  return value.bytes;
}
export function validateManifest(value) {
  if (
    !exact(value, [
      "schema",
      "adapter",
      "archive",
      "objects",
      "references",
      "snapshot",
      "binary_sha256",
    ]) ||
    value.schema !== 1 ||
    !["sqlite", "redb"].includes(value.adapter) ||
    !/^[a-f0-9]{64}$/.test(value.binary_sha256)
  )
    throw Error("invalid recovery manifest");
  if (
    !exact(value.references, ["object_store", "identity", "credential"]) ||
    value.references.object_store !== "attachments" ||
    value.references.identity !== "authentik" ||
    value.references.credential !== "synthetic-oidc-client"
  )
    throw Error("unresolved recovery references");
  if (
    !exact(value.snapshot, [
      "resources",
      "tombstones",
      "attachments",
      "post_snapshot_mutations",
    ]) ||
    value.snapshot.resources !== 1000 ||
    value.snapshot.tombstones !== 100 ||
    value.snapshot.attachments !== 20 ||
    value.snapshot.post_snapshot_mutations !== 20
  )
    throw Error("unsupported recovery profile");
  if (
    !Array.isArray(value.objects) ||
    value.objects.length !== 20 ||
    new Set(value.objects.map((v) => v?.name)).size !== 20
  )
    throw Error("incomplete snapshot object inventory");
  const bytes =
    object(value.archive, true) +
    value.objects.reduce((sum, v) => sum + object(v), 0);
  if (bytes > maxBytes || value.objects.some((v) => v.bytes > 65536))
    throw Error("snapshot byte bound");
  return value;
}
