// Verify the exact historical dependency graph before compilation.
import { readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";

const [acceptedLock, resolvedLock, metadataFile, outputFile] = process.argv.slice(2);
assert.ok(outputFile, "usage: check-lock accepted.lock resolved.lock metadata.json output.json");
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
function packages(bytes) {
  return bytes.toString("utf8").split("[[package]]").slice(1).map((section) => {
    const value = (key) => section.match(new RegExp(`^${key} = "([^"]+)"`, "m"))?.[1];
    return { name: value("name"), version: value("version"), source: value("source") ?? null, checksum: value("checksum") ?? null };
  });
}
const acceptedBytes = readFileSync(acceptedLock);
const resolvedBytes = readFileSync(resolvedLock);
const accepted = packages(acceptedBytes);
const resolved = packages(resolvedBytes);
const metadata = JSON.parse(readFileSync(metadataFile));
const graph = new Set(metadata.resolve.nodes.map((node) => node.id));
const resolvedPackages = [];
for (const item of metadata.packages.filter((item) => graph.has(item.id))) {
  if (item.name === "rom-runtime-predecessor-writer") continue;
  const selected = resolved.find((entry) => entry.name === item.name && entry.version === item.version && entry.source === (item.source ?? null));
  assert.ok(selected, `resolved lock missing ${item.name} ${item.version}`);
  assert.ok(accepted.some((entry) => JSON.stringify(entry) === JSON.stringify(selected)), `historical dependency mismatch ${item.name} ${item.version}`);
  resolvedPackages.push(selected);
}
writeFileSync(outputFile, JSON.stringify({
  acceptedLockSha256: sha256(acceptedBytes),
  resolvedLockSha256: sha256(resolvedBytes),
  metadataSha256: sha256(readFileSync(metadataFile)),
  packages: resolvedPackages,
  packageCount: resolvedPackages.length,
  status: "exact accepted dependency versions, sources, and checksums matched",
}, null, 2) + "\n");
console.log(`verified ${resolvedPackages.length} historical dependency packages`);
