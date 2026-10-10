import test from "node:test";
import assert from "node:assert/strict";
import { loadBuildPlan } from "./build-plan.mjs";
test("storage-stage diagnostics cannot reuse the default admission build", () => {
  const diagnostic = loadBuildPlan("stage-diagnostic");
  const release = loadBuildPlan("release");
  assert.equal(diagnostic.diagnostic, true);
  assert.match(
    diagnostic.command,
    /--release --features storage-stage-timings /,
  );
  assert.notEqual(diagnostic.target, release.target);
  assert.notEqual(diagnostic.executable, release.executable);
  assert.doesNotMatch(release.command, /--features/);
  assert.equal(release.diagnostic, false);
});
test("optimized plan changes compiler profile only with separate bounded target", () => {
  const dev = loadBuildPlan("dev"),
    release = loadBuildPlan("release");
  assert.equal(
    dev.executable,
    "/workspace/ROM/target/application-load/debug/rom-application-load-authoring",
  );
  assert.equal(
    release.executable,
    "/workspace/ROM/target/application-load-release/release/rom-application-load-authoring",
  );
  assert.match(release.command, /--release/);
  assert.match(release.command, /900s/);
  assert.match(release.command, /--locked --offline/);
  assert.equal(release.planned_target_bytes, 8 * 1024 ** 3);
  assert.equal(release.hard_quota, false);
  assert.throws(() => loadBuildPlan("custom"), /closed compiler profile/);
});
