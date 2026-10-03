# Standalone Wasmi capability probe

Experimental engine fixture only: no ROM dependency, adapter or integrated WASM support.

```sh
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/var/tmp/rom-wasmi-probe-target cargo test --locked --manifest-path experiments/wasmi-capability-probe/Cargo.toml
```

Wasmi 2.0.0 runs actual WAT guests. Nine tests cover fuel exhaustion, missing
host import, memory growth denial, valid bounded JSON proposal, oversized length,
invalid pointer/range, and malformed/duplicate/unknown/invalid proposal values.
The 64-byte output and 64KiB memory limits are fixture choices, not production defaults.
Only in-memory guest execution and host parsing occur. This does not test Wasmtime,
components/WIT, external I/O cancellation, malicious compilation cost or ROM authority.
Debug info is disabled and the standalone workspace has its own lockfile.

An ignored release observation separates textual WAT parsing/module construction,
fresh-store instantiation with a cached module, warm calls with validation,
cached typed-export calls alone, and host JSON/schema checks alone:

```sh
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/var/tmp/rom-wasmi-probe-target cargo test --locked --release --manifest-path experiments/wasmi-capability-probe/Cargo.toml repeated_release_cost_observation -- --ignored --nocapture
```

`release-observation.log` records five samples on 2026-10-03 in the native
`rom-dev` container, Rust 1.99.0 (b940084d7 2026-09-28). Each sample has 200
module/instance iterations and 2,000 warm-call/validation iterations; lazy
translation is warmed before timing. Reported values are aggregate sample means,
not individual-call latency percentiles. Timings include loop/drop overhead.
Fresh instantiation does not include running a proposal. The no-validation call
case differs in work and cannot establish a speedup ratio for the full path.
The static 13-byte JSON/WAT fixture is not representative of Rust guests or
production traffic. Allocation counts, peak RSS and cache growth are unmeasured.
