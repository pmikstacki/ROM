# Process supervision option admission

Date: 2026-10-04. Source baseline: `cc7575bd63c8241a4e2d9c93bec362ab88f1ec47`.
The tests ran with changes to `scripts/skills/process.mjs` and its test module.
This record concerns the shared local process runner, not Resource mutation semantics.

## Failure and correction

A separate filesystem probe supplied `timeoutMs: 30000` to the runner.
The runner accepts `timeout`, so it used its 180-second default instead.
The probe completed without a timeout. It did not establish a configured 30-second deadline.
The original probe record and a separate correction remain in the disk-coordination evidence directory.

The first regression test reproduced silent acceptance of an unknown option.
The second reproduced Node's timer overflow: `2147483648` milliseconds became a 1-millisecond timeout.
The [pinned Node.js 22.16.0 source](https://github.com/nodejs/node/blob/v22.16.0/lib/internal/timers.js#L127-L175) defines that limit and conversion.
The retained [option failure](evidence/rom-0.0.2/process-admission/options-red.log) and [timer failure](evidence/rom-0.0.2/process-admission/timer-range-red.log) identify these conditions.

The runner now accepts only `cwd`, `env`, `timeout`, and `maxBytes` option keys.
An explicit numeric limit must be a positive safe integer.
The timeout must also fit Node's supported timer range, through `2147483647` milliseconds.
Invalid options fail before process creation. Undefined limits retain the existing defaults.
This correction does not change valid callers, process-group ownership, or the fixed release command sequence.

## Executed checks

The [combined result](evidence/rom-0.0.2/process-admission/regression.log) records **57 passing tests** with Node.js 22.16.0.
The command used the pinned Rust toolchain for package fixtures; it did not run workspace compilation or the release producer.

```sh
node --test scripts/skills/*.test.mjs scripts/packages/*.test.mjs scripts/release-artifacts/*.test.mjs
```

The admission test checks unknown options, invalid limits, timer overflow, undefined defaults, and absence of the rejected child's marker file.
Existing process tests verify termination of a descendant with inherited output handles and enforcement of the output-byte limit.
Package and artifact fixtures verify their existing contracts with this shared runner.
These fixtures do not replace the complete eight-gate release acceptance.

## Limits

This correction does not bound aggregate disk allocation or processes that escape a process group.
The separate confinement experiment tests private filesystem and PID namespace boundaries.
The full producer remains deferred until its capacity and write boundary are established.

The prose uses the repository's STE fallback rules. It does not certify compliance with the official dictionary.
