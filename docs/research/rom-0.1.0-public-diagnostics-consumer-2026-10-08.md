# Public diagnostics host consumer

Date: 2026-10-08. Status: executed workspace consumer tests; installed-package admission remains open.

The external consumer uses only public `rom`, `rom-consumer` and `rom-sqlite` imports.
Its new test file is `examples/consumer/tests/diagnostics.rs`.
The existing package workflow copies consumer tests and executes them from extracted library sources.
That workflow has not yet executed these new tests against the final release artifacts.

## Executed behavior

| Scenario | Oracle |
| --- | --- |
| Create, receipt replay and action with host drainage | Two commits, one stored effect, matching replay correlation and no private payload in serialized records |
| Full buffer, then closed reader | Commands and shutdown complete within the two-second test ceilings; three Resources persist |
| Host exporter waits, then returns BrokenPipe | Another command and shutdown complete before exporter release; both Resources persist |

The stalled exporter holds only the public reader. It does not retain the Runtime or database.
The host supplies exporter ownership and failure handling; the core does not execute the exporter.
The BrokenPipe result is a deterministic host fixture, not a production network exporter.

Redaction assertions include the title, note, principal, Resource ID and request keys.
The serialized records contain fixed diagnostic categories and opaque correlation tokens.
Replay retains the same logical operation and root tokens; the local record sequence advances.
This test does not claim that telemetry sequence establishes journal order.

The fixture uses an explicit synthetic key and session. It does not supply production key material.
The single-adapter consumer test uses SQLite; the maintained diagnostics conformance suite separately covers SQLite and redb.

## Commands and evidence

The commands used the NixOS development container, two build jobs and disabled incremental compilation.
Each Cargo command had an explicit 120-second ceiling.

```sh
cargo test --offline --locked -p rom-consumer --test diagnostics
cargo clippy --offline --locked -p rom-consumer --test diagnostics -- -D warnings
```

The first compile failed because `Snapshot<Task>` does not implement `PartialEq`.
The test now compares its public ID, revision and value separately.
The corrected invocation passed all three tests. Scoped Clippy passed with warnings denied.
The original compile failure remains preserved; it is not a behavioral RED.

| Input or evidence | SHA-256 |
| --- | --- |
| Test source | `d8a595a180e93240f541a100f0c8d21906541912ff505d7e2db214d839096c75` |
| Root lock | `a58a4ccca73fee85836e6f6a8052d59432f92f10dbd03fabfe09d6111b149ae4` |
| Executed native test binary | `1a8b99ed85fc7abbb9201e5e863cbe869f793b60874cd89c5d6d9afeb283cb46` |
| Initial compile log | `43ccc9bc9e6df5f0036c8d02bc584b6ce557a08cd75ad4f5990d9393440c2c40` |
| Corrected test log | `e7e48d2c585b0652280ddd5cdfcc5a4cc42aa8d75d48782bfef9dc9c46b869c0` |
| Clippy log | `96985f61f579ddbd2d58e2768b104fbb1127d7f587b72f45cbc861365a83bac2` |

Native logs are `/var/tmp/rom-010-public-consumer-diagnostics-{first,corrected,clippy}.log` inside the development container.
The native test binary is `/workspace/ROM/target/debug/deps/diagnostics-f0d665d3483f4932`.

## Remaining limits

Final-source overhead, the full local verifier and independent release acceptance remain open.
An installed consumer must repeat this drainage contract against the matching extracted release packages.
Production exporters also need their own transport redaction, deadlines, disposal and host resource limits.
These tests do not provide those guarantees for arbitrary exporters.

## Current-source repetition and overhead

The coordinator repeated the public consumer tests and scoped Clippy after the current AI checkpoint froze.
All three public diagnostics tests passed. Clippy completed with warnings denied.
The native commands used the development container, two build jobs, no incremental compilation and individual 120-second ceilings.
The evidence capsule is `/root/ROM/.superpowers/rom-010-final-diagnostics-6Ffo1E`.
Its source fence covers 591 files and confirms no source change during the checks.

The explicitly selected overhead test passed and emitted all 18 measurements.
The matrix has three rounds for each adapter and each mode: disabled, host-drained and undrained.
Each round creates 64 Resources and replays 512 receipts.

| Adapter | Disabled wall time, summed rounds | Drained change | Undrained change |
| --- | --- | --- | --- |
| SQLite | 1,289,761,262 ns | +6.37% | +4.06% |
| redb | 1,412,191,554 ns | +6.96% | +4.50% |

The fixed mode order and unoptimized test profile limit these local comparisons.
Drained mode includes host reader work. These results do not establish a production overhead bound or the larger R10 workload.
The first parser omitted the measurement prefixed by Rust's inline test name.
Its incorrect 17-row summary remains preserved as `acceptance.json`; it is superseded by `acceptance-corrected.json`.
The corrected parser requires the exact 18-row matrix before producing this summary.
The raw native log is `/var/tmp/rom-010-diagnostics-final-source-overhead.log`.
The public consumer and Clippy logs use `/var/tmp/rom-010-diagnostics-final-public-consumer.log` and `/var/tmp/rom-010-diagnostics-final-public-clippy.log`.
Final installed-package verification and the complete release verifier remain open.
