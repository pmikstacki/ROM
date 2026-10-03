# Core ergonomics, resilience and generic CLI

## Authoritative goal and scope

Owner steering on 2026-10-02 replaces the earlier all-directions goal: complete
research of deferred capabilities, improve API ergonomics and core resilience,
and provide CLI instead of Studio. The goal-status tool currently permits status
changes only, so its earlier objective text is superseded by this instruction and
this design. Do not implement Studio or treat research as implementation of every
future capability. This goal completes when the indexed research, maintained
improvements, CLI, combined tests/review and publication are complete.

## Selected approach and alternatives

Use an optional Rust HTTP CLI over the existing semantic transport. A generic
embedded plugin CLI would couple each binary to application declarations; an
application-specific command set would duplicate the domain. HTTP already
supports invocation, query, live snapshots and durable journal batches, so the
client can serve every registered kind. Native discovery belongs in core with a
thin HTTP route. Trusted descriptors cannot be exposed directly. Row-dependent
policies cannot establish static permissions.

## Public core additions

`Query::<R>::all()` and `Default` construct `QuerySpec::all()` through the existing
authorized bounded selection; `.and`, `.limit`, `.after_id` retain semantics.
Core concurrency counts (`actions`, `io_jobs`, `subscriptions`) and HTTP `bodies`
validate `0 < value <= tokio::sync::Semaphore::MAX_PERMITS` before construction.
Existing zero/error behavior is preserved; invalid maximum values return an error.

Add `DiscoveryTarget<'a> { Resource, Field(&'a str), Action(&'a str) }` and
`Definition<R>::discovery_policy(F)` where F is a trusted Send+Sync predicate over
Actor and DiscoveryTarget. Default denies all. Resource grant is necessary before
field/action grants. `Runtime::discover(&Actor) -> Result<Discovery>` is async and
uses existing observe/current-authority/generation/byte-budget guarantees.

The serialized contract is `Discovery { version: 1, resources: Vec<DiscoveredResource> }`,
with each resource `{ kind, version, fields, actions }`; fields `{ name, shape }`,
actions string names. Serialize Shape with tagged `type` and `value`, snake_case;
Reference value is `{kind}` and recursive containers carry nested shapes. Sort
kinds, fields and actions deterministically. No registration totals, grant flags,
policy names, source ownership, omission markers or arbitrary defaults/values.
Conservatively omit fields containing references to hidden kinds. Action input
schemas remain opaque: Input currently defines codecs, not public schema.

No row scan, row-policy evaluation against synthetic values, codec or action
execution occurs during discovery. Discovery is metadata disclosure, not operation
authorization. Registry callbacks can panic. Existing supervised error handling
must convert a panic into a safe error. The entire serialized response fits the snapshot byte
limit or fails with TooLarge. HTTP POST `/discover` uses existing authenticated bounded
body handling and the same core method. Empty object request only.

## CLI contract

Package `rom-cli`, binary `rom`, optional workspace member. Commands operate on
known kind IDs and do not require discovery to execute. Commands: discover [KIND],
read KIND ID, query KIND, create/replace/patch/delete KIND ID, action KIND ID NAME,
invoke (exact invocation file), live KIND, journal KIND, journal-head KIND,
subscribe KIND. JSON source flags are `--input-file`, `--query-file`,
`--request-file`, `--after-file`; `-` means stdin. Query defaults to all. Mutations
require explicit `--idempotency` and non-create operations `--expected`; invoke
keeps the original envelope unchanged. Do not supply destructive force defaults.

Connection uses explicit `--endpoint` and optional `--auth-file` containing the
complete Authorization value. HTTPS verifies TLS; HTTP is numeric loopback only
in this profile. Reject URL userinfo/query/fragment, header CR/LF, and oversized
credentials. Trim one trailing file newline to support ordinary secret files.
Never print credentials or request payloads in diagnostics. No redirects, automatic
network retries, proxies or credential persistence. No synthetic login or inferred
principal identity. Backend-origin messages are mapped to documented categories,
not raw untrusted error strings.

Request JSON (64 KiB maximum) rejects duplicate keys at every nesting level before
Value conversion. Finite response and SSE frame limits are 2 MiB. Connect/request
and stream inactivity limits are finite/configurable within validated bounds;
SSE keepalive activity counts as activity, not domain progress. Consume frames
incrementally across arbitrary chunk boundaries, CRLF and multiline data. If frames are malformed or oversized, or the stream is lost, stop. Do not resume automatically. Live snapshots and
journal batches remain distinct; no cursor for live. Preserve complete journal
batch/cursor association, including empty batches. Never skip a history gap.

`--output human` is default, with escaped field values and kind/id/revision;
`--output json` finite commands emit one JSON value, streams one complete frame
per line. Diagnostics go to stderr. Bound memory. Flush each complete frame.
If the stdout pipe closes, terminate cleanly. No terminal control sequences from data.
Ctrl-C stops waiting/observing; it does not assert accepted mutations rolled back.

Exit codes: 0 success/intentional stream stop, 2 usage/local validation, 3 explicit
remote rejection for reads/streams, 4 transport/protocol failure for reads, 5 mutation outcome
unresolved, 6 history gap, 130 interrupt while waiting for a mutation (uncertain).
After submission, every non-success mutation response or transport/protocol
failure is classified as unresolved. HTTP error categories carry no execution
phase: even Invalid, Conflict, TooLarge or NotCommitted may originate from a
trusted ActorGate during post-commit observation. Do not infer rollback from a
category or status. Describe unresolved outcome and
same-principal replay with original key/revision/input. No automatic replay or
credential-scoped recovery store. The client cannot certify an unchanged principal.

## Validation and compatibility

Keep one CLI path for two unrelated resource kinds; use actual binary + TCP
against both adapters. Cover cancellation, duplicate JSON, partial values,
canonical custom fields, idempotent retry/mismatch, revocation, hidden metadata,
stream boundaries/limits, history gap, redirects and lost acknowledgments.
No new storage format or silent replay behavior. Compile-only success is not the
end-to-end acceptance. Preserve Rust 1.99 and MIT source distribution; audit any
new dependencies and include their notices.
