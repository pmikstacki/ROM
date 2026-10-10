import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, realpathSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { isAbsolute, join, relative, resolve, sep } from "node:path";
import { captureLoadSources, verifyLoadSources } from "./source-witness.mjs";
const root = realpathSync(fileURLToPath(new URL("../../", import.meta.url)));
const directory = `${root}/tests/application-load`;
function maintainedCheckoutPath(checkout, sourcePath) {
  if (!isAbsolute(sourcePath) || resolve(sourcePath) !== sourcePath) return false;
  const local = relative(checkout, sourcePath);
  if (!local || isAbsolute(local)) return false;
  const components = local.split(sep);
  return components.every(
    component => component !== ".." && component !== ".superpowers" && component !== "node_modules",
  );
}
test("load fence includes every maintained harness module and five Studio parsers", () => {
  const sources = captureLoadSources();
  const paths = sources.map(({ path }) => path);
  for (const name of readdirSync(directory).filter((name) =>
    name.endsWith(".mjs"),
  )) {
    assert.ok(paths.includes(`${directory}/${name}`), name);
  }
  for (const name of [
    "codec",
    "normalization",
    "serialization",
    "validation",
    "work-validation",
  ]) {
    assert.ok(paths.includes(`${root}/studio/src/lib/client/${name}.ts`), name);
  }
  assert.equal(new Set(paths).size, paths.length);
  assert.ok(paths.every(sourcePath => maintainedCheckoutPath(root, sourcePath)));
});
test("load path predicate admits private checkout parents and rejects local escapes or private inputs", () => {
  const nested = join(root, ".superpowers", "review", "workspace");
  assert.ok(maintainedCheckoutPath(nested, join(nested, "crates", "rom", "src", "lib.rs")));
  for (const checkout of [root, nested]) {
    for (const sourcePath of [
      checkout,
      join(checkout, "..", "outside.rs"),
      join(`${checkout}-sibling`, "src", "lib.rs"),
      join(checkout, ".superpowers", "private.rs"),
      join(checkout, "crates", ".superpowers", "private.rs"),
      join(checkout, "node_modules", "dependency.js"),
      join(checkout, "crates", "node_modules", "dependency.js"),
      "relative.rs",
    ]) assert.equal(maintainedCheckoutPath(checkout, sourcePath), false, sourcePath);
  }
});
for (const path of [
  `${directory}/mixed-run.mjs`,
  `${root}/studio/src/lib/client/work-validation.ts`,
]) {
  test(`load fence rejects changed or missing input: ${path}`, () => {
    const sources = captureLoadSources();
    assert.ok(sources.some((entry) => entry.path === path));
    const changed = sources.map((entry) =>
      entry.path === path ? { ...entry, sha256: "0".repeat(64) } : entry,
    );
    assert.throws(() => verifyLoadSources(changed), /source fence changed/);
    assert.throws(
      () => verifyLoadSources(sources.filter((entry) => entry.path !== path)),
      /source fence changed/,
    );
  });
}
