# ROM 0.1.0: verification after publication

Date: 2026-10-10. Status: complete local verifier passed; production acceptance remains incomplete.

## Verified source and command

The unchanged `./scripts/check` passed on published tag `v0.1.0`.
The executed commit was `d7f9ea4f0f2e9ed6e510fcbe7cd199ea3b02a397`.
The command started at 20:01:42 UTC and completed at 20:07:48 UTC.
Its physical exit code was 0, with no signal.
All 3,653 tracked files had the same content before and after execution.
Their combined content hash was `b9360215fe2f0a02651c2afbb0692b5a004d034729d64e6064ec326dad9aeba5`.
The untracked `website/` directory was preserved and excluded from this source identity.

The verifier passed 603 JavaScript tests.
Rust formatting, Clippy, workspace tests, feature checks, documentation, compile fixtures, examples, auth and identity checks completed.
The raw Rust output contains 210 passing summaries and 25 ignored test executions.
These include repeated feature checks; they are not a count of unique tests.
Ignored tests remain unverified by this command.

## Earlier attempts and execution limits

The first post-publication attempt passed 600 JavaScript tests and failed three temporary-directory checks.
Its runner supplied a path that exceeded the browser socket path budget.
The successful attempt used a short, private directory under `/var/tmp`.
The verifier and product source did not change.
A separate attempt to create a directory under the container's `/tmp` failed because that filesystem was full.
These failures remain preserved.

The owned run used limits of four CPUs, 4 GiB memory and 512 processes.
The command deadline was 900 seconds.
Target allocation grew by 463,986,688 bytes, below the monitored 8 GiB growth limit.
Final shared free space was 135,166,111,744 bytes, above the local 2 GiB floor.
Capacity monitoring was not a filesystem quota or a bound on other projects.
The memory controller recorded 1,660 limit encounters and no OOM kills.
This result does not establish execution without memory pressure.

Independent review confirmed the unchanged source, raw command completion and empty final process cgroup.
Local evidence is preserved in `.superpowers/rom-010-published-tag-check-vartmp-20261010/`.
Independent evidence is preserved in `.superpowers/rom-010-published-tag-independent-review-20261010/`.
The combined log SHA-256 is `935e95be2e9bca8d4f2d952b27cf3e25b73ca30834452f2afb807931203544f6`.
The independent evidence seal is `3bec94bdce96409d5157ac0fba288fdf384331f87dadb9eda0379d2fab62987e`.
These private evidence directories are not part of the published source archive.

## Remaining acceptance

This result closes the missing complete-local-verifier result for the published tag.
It does not close the failed mixed workload, updated preview deployment or complete application acceptance.
Production qualification still requires the deployment, recovery, upgrade and load evidence defined in the [consumer research plan](rom-0.1.0-consumer-research-plan.md).
The release remains experimental.
The published tag and historical failure records remain unchanged.
