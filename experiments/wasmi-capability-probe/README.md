# Standalone Wasmi capability probe

Experimental engine fixture only: no ROM dependency, adapter or integrated WASM support.

```sh
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/var/tmp/rom-wasmi-probe-target cargo test --locked --manifest-path experiments/wasmi-capability-probe/Cargo.toml
```

Wasmi 2.0.0 runs actual WAT guests. Seven tests cover fuel exhaustion, missing
host import, memory growth denial, valid bounded JSON proposal, oversized length,
invalid pointer/range, and malformed/duplicate/unknown/invalid proposal values.
The 64-byte output and 64KiB memory limits are fixture choices, not production defaults.
Only in-memory guest execution and host parsing occur. This does not test Wasmtime,
components/WIT, external I/O cancellation, malicious compilation cost or ROM authority.
Debug info is disabled and the standalone workspace has its own lockfile.
