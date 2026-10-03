# Operator recovery results

Date: 2026-10-03. Status: release task 4.2 implementation, independent review and combined verification complete.

## Scope

Release task 4.2 adds authorized work inspection, atomic retry and verified reconciliation.
An operator control changes existing infrastructure work. Resource actions remain the path for application changes and explicit compensation.
The [operator guide](../operator-recovery.md) describes the public API, CLI procedures, limits and host policy choices.

The source baseline is `62285bd40b7890000f89ee8fed2cba371053ada8`.
This result covers the subsequent operator recovery changes, including native format 8 and archive format 6.
The preceding ownership implementation remains part of the combined Runtime contract.
The [source manifest](evidence/operator-recovery-2026-10-03/source-files.sha256) identifies 402 source, configuration and verification files.
The [source status](evidence/operator-recovery-2026-10-03/source-status.txt) records the worktree changes.
[Environment details](evidence/operator-recovery-2026-10-03/environment.txt) identify the compiler and container.
[Evidence checksums](evidence/operator-recovery-2026-10-03/evidence.sha256) cover the retained logs and review records.

## Contracts and implementation

Every accepted work transition advances its revision. Operator requests bind the storage generation, revision, opaque work handle and exact operation identity.
The native transaction commits the control transition and operator receipt together.
An exact replay returns the receipt before stale-version checks, after current authority and retry epoch checks.
The same identity with changed input is rejected. A lost acknowledgement remains an unknown outcome until the exact receipt resolves it.

The host supplies an `OperatorAuthorizer`. The default denies inspection, retry and reconciliation.
Operator authority does not grant the original service additional Resource or field access.
Views omit frozen Resource values, action inputs, service principal keys and causal root identities.
Moving cursors bind the storage generation and query filters. Each page applies current authority before its visible limit.

Runtime owns accepted controls through its existing supervised execution path.
Provider verification runs outside the core gate and native transactions.
After verification, Runtime checks current authority and arbitrates an existing exact receipt before attempting a new transition.
Callback timeout handling requires cooperative async code; it cannot preempt arbitrary blocking Rust code.

Delivery profiles remain explicit host choices. `AtLeastOnce` is the compatibility default.
`ProviderDeduplicated` preserves a delivery ID for a provider that implements deduplication.
`ReconcileBeforeRetry` holds uncertain started deliveries, including after process recovery.
An initial provider lookup miss remains `Unresolved`; it does not permit another send.

Trusted `Accepted` verification completes work without delivery. Terminal `NotAccepted` verification permits scheduling only within the original budgets.
Neither ordinary retry nor reconciliation resets consumed attempts, causal work, original start time or chain age.
Compensation remains an explicitly registered Resource action with its own revision and mutation idempotency key.

Native SQLite and redb share the same transition and receipt validation code.
Old native formats 3–7 and archive formats 1–5 use explicit conversion paths.
Conversion gives old work revision zero, the compatibility delivery profile and an empty operator ledger.
Current decoders reject missing required metadata. Backup, restore and maintenance preserve operator evidence.

Implementation uses named modules for protocol, authorization, projection, execution, shared transitions and adapter persistence.
Crate facades contain declarations and exports. Shared callback supervision, frozen-action receipt resolution and native test helpers remove duplicate behavior.

## Focused acceptance

| Area | Executed check |
| --- | --- |
| Revision and atomicity | Rejected transitions and overflow leave state unchanged. Native faults before commit preserve state; lost post-commit acknowledgement recovers an exact receipt. |
| Native process recovery | SQLite and redb subprocess fixtures interrupt persisted work and controls. Reopen recovers the durable outcome without duplicating the accepted transition. |
| Authority | Default denial, actor revocation, service/source permission changes and saved-scope replay checks remain enforced. |
| Protocol and bounds | Unknown fields, invalid versions, malformed results, excessive input and response limits fail without partial success output. |
| Hold profile | Timeout, panic, unknown outcomes and interrupted started delivery require reconciliation before another send. |
| Verification | Accepted, terminal NotAccepted, Unresolved, timeout, panic, malformed evidence, concurrent exact requests and authority races exercise both adapters. |
| CLI integration | The actual CLI uses loopback HTTP to inspect a hold, reject ordinary retry, observe Unresolved, complete through verification and replay without another send. |
| Application journey | The demo restarts stopped work, schedules it, replays the operator receipt, then invokes its existing compensation action idempotently. |

The independent reconciliation target contains 19 tests, including its subprocess helper.
The CLI operator target contains 14 tests. These counts are test functions, not counts of distinct failure scenarios or database runs.
Provider callbacks in this stage are controlled fixtures. They establish Runtime behavior, not external-provider delivery guarantees.

## Findings and corrections

A review found that valid result identity alone did not reject an impossible operation/outcome pair after commit.
For example, a malformed adapter result could claim `Completed` for a retry control.
Shared result and receipt validation now rejects this as an unknown outcome. Repeating the original request recovers the genuine receipt.
The CLI uses the same public correspondence validator.

Serde can deserialize some structs from positional arrays. HTTP now requires an object for public request envelopes.
CLI operator files and responses have the corresponding shape check. Nested Resource arrays remain valid.

The initial profile tests reproduced repeated delivery after uncertainty and process interruption.
The shared lifecycle now preserves the explicit reconciliation hold across completion, expired claims and restore.
Concurrent identical verification calls arbitrate their atomic receipt before checking a stale version.

The CLI integration test initially treated the caller's lookup reference as private provider evidence.
That assertion was incorrect: the result deliberately repeats the submitted operation for identity verification.
The corrected test checks that actual provider evidence and frozen payloads remain absent.
The guide prohibits secrets in the lookup reference.

Documentation review also qualified timeout claims. ROM aborts and joins a cooperative callback; a blocking or non-yielding callback can delay completion.
This is a host callback contract, not a new guarantee of preemptive execution.

## Combined verification and review

The coordinator ran this sequence in `rom-dev` on the final source tree:

```sh
export CARGO_TARGET_DIR=/var/tmp/rom-release-measured-verification-target
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_BUILD_JOBS=2
./scripts/check
./demo/verify
./demo/run operator sqlite
./demo/run operator redb
./demo/run upgrade sqlite
./demo/run upgrade redb
```

The complete sequence exited with status zero. The [combined log](evidence/operator-recovery-2026-10-03/combined-check.log) retains the results.
It includes strict OpenSpec validation, formatting, Clippy, workspace tests, documentation, compile fixtures, core dependency isolation and auth/identity checks.
Demo verification also tests serve, reopen and SIGINT drain on both stores.
The source manifest was checked again after execution; all 402 hashes matched.

An earlier combined run exposed a stale HTTP fixture that expected retry despite missing source access for its service.
The corrected fixture tests both refusal and explicit named-service read permission. No product rule was relaxed.
Its focused HTTP and Clippy checks passed before the successful combined run.
A separate earlier run found module declaration ordering; rustfmt corrected it. Both failure logs remain in the evidence directory.

Independent reviews accepted the protocol, shared/native persistence, compatibility, Runtime, delivery profiles, HTTP, CLI and application journey.
The Runtime reviewer reported no code findings; the design now explicitly permits repeating an unchanged inconclusive verification request.
Focused test counts and reviewer inspection are separate from the coordinator's full execution evidence.

## Completion boundary

The operator implementation does not change the default delivery policy or introduce human attestations that force provider outcomes.
The bounded operator ledger retains evidence and refuses new controls at capacity; it does not silently evict receipts.
Process-exit tests do not certify hardware behavior under power loss.

Real-provider identity/bootstrap/secrets, extension conformance and final package acceptance remain later release tasks.
