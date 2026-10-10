import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, realpathSync } from "node:fs";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { captureLoadSources, verifyLoadSources } from "./source-witness.mjs";
const directory = realpathSync(fileURLToPath(new URL("./", import.meta.url)));
test("actual load source fence includes every v2 reader, selector and regression input", () => {
  const sources = captureLoadSources();
  for (const name of [
    "lease-streams.mjs",
    "lease-streams.test.mjs",
    "lease-streams-review.test.mjs",
    "lease-streams-row-shape.test.mjs",
    "lease-streams-source-witness.test.mjs",
    "mixed-run.mjs",
    "mixed-v2.test.mjs",
    "mixed-lease-protocol.mjs",
    "mixed-lease-protocol.test.mjs",
  ]) {
    const path = directory + "/" + name,
      entry = sources.find((value) => value.path === path);
    assert.ok(entry, name);
    assert.equal(
      entry.sha256,
      createHash("sha256").update(readFileSync(path)).digest("hex"),
    );
  }
});
for (const name of [
  "lease-streams.mjs",
  "mixed-run.mjs",
  "mixed-lease-protocol.mjs",
])
  test(`v2 executable source drift or omission refuses load: ${name}`, () => {
    const sources = captureLoadSources(),
      path = directory + "/" + name;
    assert.ok(sources.some((value) => value.path === path));
    assert.throws(
      () =>
        verifyLoadSources(
          sources.map((value) =>
            value.path === path ? { ...value, sha256: "0".repeat(64) } : value,
          ),
        ),
      /source fence changed/,
    );
    assert.throws(
      () => verifyLoadSources(sources.filter((value) => value.path !== path)),
      /source fence changed/,
    );
  });
