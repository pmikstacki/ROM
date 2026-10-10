export function loadBuildPlan(profile) {
  if (!["dev", "release", "stage-diagnostic"].includes(profile))
    throw Error("closed compiler profile");
  const diagnostic = profile === "stage-diagnostic",
    release = profile !== "dev",
    target = diagnostic
      ? "application-load-stage-diagnostic"
      : release
        ? "application-load-release"
        : "application-load",
    maximum = release ? 900 : 120;
  return Object.freeze({
    profile,
    diagnostic,
    target,
    maximum_seconds: maximum,
    planned_target_bytes: 8 * 1024 ** 3,
    hard_quota: false,
    executable: `/workspace/ROM/target/${target}/${release ? "release" : "debug"}/rom-application-load-authoring`,
    command: `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_RELEASE_DEBUG=0 timeout --signal=TERM --kill-after=5s ${maximum}s cargo build --manifest-path tests/application-load/host/Cargo.toml --locked --offline ${release ? "--release " : ""}${diagnostic ? "--features storage-stage-timings " : ""}--target-dir /workspace/ROM/target/${target} --message-format=json`,
  });
}
