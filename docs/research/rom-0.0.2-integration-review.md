# ROM 0.0.2 integration review

Date: 2026-10-04 UTC.
Fixed point: `91954b6`.

This review covers the implemented Studio metadata, SDK, current host authority, author workflow, and release integration.
It found several concrete correspondence gaps. The authors repaired the SDK and attachment findings with regressions.
The later shared file-admission correction closes the remaining demo finding.
The [source record](evidence/rom-0.0.2/integration-review/source.json) identifies the inspected boundaries and host context.
It does not imply an exhaustive review of every changed file since the fixed point.
The [final source record](evidence/rom-0.0.2/integration-review/final-source.json) binds the correction and current inspected inputs.

The standards review applies [the quality gates](../quality.md).
The contract review applies the [Studio design](../superpowers/specs/2026-10-03-rom-0.0.2-studio-design.md)
and [implementation plan](../superpowers/plans/2026-10-03-rom-0.0.2-studio.md).
The current UI, metadata, host, and release author records remain separate evidence sources.

## Findings and corrections

| Priority | Finding | Current result |
| --- | --- | --- |
| P2 | Discovery required one input descriptor for every disclosed action name. Native discovery can hide an input that references an unauthorized Resource kind. | Repaired: described inputs can be a subset. Unknown and duplicate inputs remain invalid. |
| P2 | SDK discovery accepted native-impossible enum, Optional, Nullable, scalar-input, and codec-name metadata. | Repaired: six finite negative probes now reject these descriptions. |
| P2 | Attachment HTTP 500 failures discarded the pending reservation and file as a confirmed refusal. | Repaired: unknown failures retain the exact submission for explicit retry. |
| P2 | The new trusted demo profile duplicates file admission with a precheck followed by blocking `File::open`. | Repaired: both callers use shared opened-handle admission. Author execution evidence was reviewed. |
| P3 | Download filenames used the current selection after an await. | Repaired: the selected ID is captured before the download. |
| P3 | QueryEditor omitted `codec_wrappers` when it delegated to a custom value editor. | Repaired: wrapper metadata is forwarded; the author reports a two-browser regression. |

The demo profile finding is in `demo/src/studio_profile.rs::bounded_file`.
The existing `provider_profile/secrets.rs` opens files with `O_NOFOLLOW | O_NONBLOCK`, then checks the opened handle.
The new helper checks `symlink_metadata`, then uses blocking `File::open` before its opened-handle checks.
A path replacement between these operations can follow a link or block on a FIFO before validation.
The current tests establish static symlink rejection. They do not establish race-safe admission.
This finding is source-backed; this reviewer did not execute a concurrent path-replacement probe.
The correction moves admission to the named `demo/src/host_files.rs` module.
Both profiles now open with `O_NOFOLLOW | O_NONBLOCK`, then validate the opened descriptor before a bounded read.
Provider callers retain their existing path, private permission, and `Denied` behavior.
Studio credentials retain absolute paths, private permissions, and process-owner UID checks.
Studio profile files retain absolute paths and regular-file checks.
Both paths reject empty, oversized, and invalid UTF-8 content.
The non-Linux fallback remains closed.

The existing public module paths stay unchanged. The crate root adds declarations only.
The Studio feature enables the already pinned optional `libc` dependency; no dependency version changes.
Studio filesystem errors intentionally use one generic `profile file admission rejected` message.
This replaces earlier specific startup messages without exposing paths or credential bytes.
The provider error remains `Denied`.
The reader still requires immutable files under host-trusted parent directories.
`O_NOFOLLOW` protects the final path component, rather than establishing trust in every parent directory.

The reviewer inspected the [author's correction evidence](rom-0.0.2-secure-host-files-results.md)
and independently matched all ten frozen file hashes with the live source.
The author executed complete demo tests with `studio,provider-profile`, including direct symlink and stable FIFO rejection.
The first combined log includes successful tests followed by a lint-style failure.
The separate final Clippy/build log records its correction and successful completion.
The author also executed trusted HTTPS-profile launcher checks on SQLite and redb.
No new native build or pathname-race experiment was run by this reviewer.
The source-backed race and duplicated-reader findings are closed.

The [original metadata probe](evidence/rom-0.0.2/integration-review/metadata-probe.log) recorded six accepted invalid descriptions.
The [corrected probe](evidence/rom-0.0.2/integration-review/metadata-fixed-probe.log) records six rejections and accepts the legitimate hidden-input subset.
The reviewer executed both probes against their recorded source.

The [attachment probe](evidence/rom-0.0.2/integration-review/attachment-probe.log) exercises five corrected response classifications.
HTTP 500, status zero, and overload preserve the pending submission.
Explicit retry submits the same reservation object and frozen file bytes.
Known `not_committed` and ordinary authorization refusals clear the pending operation.
There is one submission before explicit retry.
The reviewer also executed the [six model tests](evidence/rom-0.0.2/integration-review/attachment-model-independent.log).
The original HTTP 500 RED test and real-browser reruns were executed by the author, not by this reviewer.

## Float authoring and extension limits

The metadata probe also checks lexical float behavior.
Wire parsing and replay preserve `{"rate":0.0}`.
A generic form's JavaScript number zero serializes as `{"rate":0}` after value normalization.
These values remain equal for the built-in `FiniteF64` field.
Its native decoder uses `Value::as_f64`, accepts either numeric category, and encodes a finite float afterward.
This observation is not evidence of corruption in the built-in float field.

Native query normalization uses the accepted field codec before binding a query anchor.
The SDK keeps the returned canonical anchor string opaque and preserves its float lexemes.
The query-anchor correction has separate executable evidence in the SDK review.

A custom codec that requires a float lexical category must account for JavaScript scalar-number authoring.
The generic primitive value editor does not promise exact arbitrary scalar wire categories.
This is an extension limit, not a reason to change the current built-in float contract.
Unknown custom codecs remain read-only until the host supplies a compatible renderer.
The independent Field registry proposal remains open; Resource-owned codec descriptors do not implement that registry.

## API, authority, and module boundaries

Resource descriptors retain stable field and action identities.
Action input metadata is a disclosed subset, rather than a grant of mutation authority.
The browser uses disclosed descriptors and codec identities; native registration and current policies still decide admission.
The query editor's wrapper correction restores the same custom-renderer contract used by field editing.
The declared native shape limits now apply at SDK discovery admission.

The host obtains Human authority from sealed provider proofs and current identity Resources.
The browser cannot create an Actor or bootstrap a User through the proxy.
Provider, Link, and User state remains subject to current checks for routes and observed output.
The client generation and request epochs prevent old responses from restoring invalidated capability state.
The prior independent blob-client review supplies executable response, cancellation, and cache-fencing probes.
This review found no additional demonstrated authority bypass in these inspected boundaries.
It does not replace the host's actual-browser acceptance or provider certification.

The root facades contain declarations and exports.
The only function entry points found in crate `lib.rs` files are the allowed procedural macro delegators.
The host, SDK, metadata, and release mechanisms use named modules by responsibility.
The shared file reader removes the concrete duplicated mechanism identified in this review.
No broad architectural rewrite is proposed.

## Release artifacts and author procedure

The artifact verifier binds the complete source inventory, Git archive commit, lockfile, selected skill identity, and extracted canonical profile.
The Studio profile additionally binds copied npm inputs, command correspondence, extracted asset hashes, and actual-host acceptance.
Historical version-one source-only verification retains its six-gate shape.
The current producer has a fixed complete profile; the caller cannot replace it with a selected skill example.

This reviewer authored the query-anchor correction, OIDC proof subset, resilience tests, and Task7 packaging code.
Review of those implementations is an own-source audit, not independent acceptance.
The coordinator's separate Node execution and the peer notice review provide independent evidence for the packaging boundary.
The [runtime notice report](rom-0.0.2-runtime-notices-results.md) records the new inventory and original notice retention.
Its real production build evidence is separate from actual-host acceptance.

The earlier release skill's manual list omitted newer build and Studio gates.
It correctly withheld release approval from the selected example, but did not identify the complete producer.
The refreshed skill now uses the supplied checkout's canonical artifact procedure and `./scripts/release`.
The repeated agent scenario identifies source prerequisites, the actual producer, both browsers, and extracted-asset acceptance.
No producer was run for that scenario. Prior author knowledge prevents a blind or human-usability claim.

## Acceptance boundary

The finite probes establish the corrected metadata and attachment invariants described above.
Browser, native-host, migration, and complete release results reported by other authors remain attributed to those authors.
The file-admission and peer notice corrections are complete at the final recorded source boundary.
This review has no remaining implementation findings in its inspected scope.
The coordinator must execute the complete release against the final clean source and retain its immutable artifacts.
