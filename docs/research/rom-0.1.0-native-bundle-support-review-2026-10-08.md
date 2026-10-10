# Independent native bundle support review

Date: 2026-10-08. Scope: production support promotion, StorageMetadata, prepare_bundle, shared bundle helpers, and focused raw metadata tests.
No native execution or product edit occurred during this review.
The owner was still adding WorkEdit poisoning and broader checks; this is a source checkpoint, not final-source approval.

## Result

The reviewed prepare_bundle path preserves the existing bundle stages without cloning retained Work or operator history.
It prepares metadata and work changes for one native transaction. It does not itself publish or authorize execution.
No additional blocker was found in this coordinator beyond the separately known public WorkEdit ignored-error hazard under correction.
Do not claim that hazard fixed until corrected source and maintained failure evidence are verified.

## Metadata split and reconstruction

StorageMetadata contains epochs, limits, generation, journal head/floor, receipt/effect counters, and bounded journal events.
It deliberately excludes retained WorkLedger and OperatorLedger collections.
split moves these components out of canonical StorageState. reassemble restores the original fields for full archive validation.
from_state clones only bounded metadata, although the caller must already hold validated canonical state.
The type has no Debug implementation and rejects unknown fields during deserialization.

Serialization alone is not integrity validation.
Native open/import must validate limits, journal, counters, epochs, canonical work, operator receipts, and derived summaries together.
Neither split nor reassemble provides that complete validation automatically.
Their documentation identifies trusted persistence support, not a transport projection.

Metadata journal reads reuse canonical cursor and bounds logic with empty retained work/operator components.
They do not invoke archive validation on that intentionally incomplete intermediate state.
Ordinary coordinator cloning is proportional to bounded journal metadata, not retained Work or operator history.
This is a source property, not a measured native speedup.

## Bundle ordering and atomicity

The adapter must arbitrate the exact receipt and Resource revision before calling prepare_bundle in its original transaction.
An exact committed receipt replay must bypass fresh completed-work claim validation, as in the existing native adapter.
prepare_bundle is for the new candidate after that arbitration; it does not implement replay itself.

The coordinator preserves these stages:

1. Check the receipt retry epoch against replay boundaries; require coherent metadata/work retry headers.
2. Validate the live causal claim and its epoch, then apply ordinary versus causal admission-floor rules.
3. Reject reaction cause epochs that differ from the receipt epoch.
4. Prepare completed Work through Finish, including its revision/accounting checkpoint before later counter errors.
5. Increment receipt/effect counters with checked arithmetic and enforce limits.
6. Require changed state and declared limits for new reactions, then enqueue through the same work overlay.
7. Prepare bounded journal changes and retired event identities.
8. Finalize work revisions once, validate touched epochs and coherence, and return the combined candidate.

The shared count_bundle and journal_bundle helpers retain the original canonical operations and error types.
Canonical StorageState::bundle still uses a private full candidate; the new metadata coordinator uses a private bounded candidate.
The returned before_metadata and work preconditions support native publication checks.
No metadata or work writes occur inside preparation.
On any propagated error, the candidate drops without changing caller state.

Native publication must atomically include Resource/reference changes, receipt/event/effect writes, work/root/index changes, and metadata.
Do not persist BundleDelta's parts independently or apply them to a different metadata snapshot with an equal work header.
The original coherent writer transaction remains mandatory, including commit-unknown fencing and lost-acknowledgement handling.

## Public Work support limits

The new storage_support facade exposes named work and metadata support paths for native adapters.
It does not replace Resource semantics or add a serialized executable delta protocol.
Work header/accounting DTO deserialization supplies data, not canonical graph integrity or authorization.
Imports must recompute accounting and validate the graph before those summaries become authoritative.

At review time, public WorkEdit methods can mutate their private overlay before returning an error.
Ignoring an error and calling finish can therefore expose a partial candidate unless the pending poison latch rejects it.
prepare_bundle itself propagates every method error immediately, so this coordinator does not reuse a failed edit.
The public composable support surface still needs the maintained ignored-error case and poison-latch correction before integration.

## Actual focused evidence

The initial metadata API stage included a compile error E0507; that is setup/compiler evidence, not a behavioral RED.
The subsequent maintained stub RED reports three PASS and two FAIL for missing bundle publication and wrong rejection behavior.
The corrected focused log reports five PASS, zero failures, and zero ignored cases.
It compares complete canonical completion-plus-child-enqueue state, rejects counter overload without publication, and checks split/journal behavior.

| Host log under `/var/tmp/` | SHA-256 |
| --- | --- |
| `rom-010-native-bundle-publication-red-20261008.log` | `24bd7aa496f1bec87ef342eaf553fe87cf39f12465414f397349675cabd7cb8d` |
| `rom-010-native-bundle-publication-green-20261008.log` | `7a421b3b257f38fbc5d67ae4dfa4af1faf8b41e8d56142f2d236efa24871567e` |

The audit read both raw logs and calculated their hashes.
The completion/enqueue comparison uses current canonical StorageState::bundle, whose count/journal helpers are shared after this refactor.
It is useful integration evidence but not an independent oracle for those shared helper operations.
Add compound error-precedence, stale claim, epoch, unchanged-state reaction, journal limit, and revision-overflow cases before native integration.
Native adapter transaction faults, format conversion, production load, full verifier, and final package evidence remain open.

## Production-support corrective checkpoint

The current public WorkEdit stores the first failed step in a private failure latch.
Later methods and finish return that failure instead of publishing the partially mutated overlay.
The actual 150-test core log includes `failed_public_editor_cannot_publish_partial_enqueue_after_ignored_error` passing.
This closes the reviewed ignored-error hazard for the maintained scenario.

StorageMetadata now exposes check_retry_epoch before Resource revision arbitration.
It checks replay-floor eligibility first and bypasses stale claim lookup when an exact receipt already exists.
For fresh work it checks coherent epoch headers, the live causal claim, matching epoch, and ordinary versus causal admission rules.
prepare_bundle shares the same new-epoch helper after native arbitration.
The native adapter must call this pre-revision seam in the existing receipt/epoch/revision order.
Preparing a bundle only after revision arbitration would still change Missing-claim versus Conflict precedence.

The core log includes passing stale-claim replay and causal-admission-below-floor differential cases.
The latter compares epoch 0/1/2/3, replay presence, and causal claim presence against canonical checks.
It also includes the new native-import projection case.
WorkImage::from_native validates canonical work and epochs, recomputes complete accounting/root/active projections, and rejects any mismatch.
It does not silently repair stored metadata. Its input collection must already be bounded by the native caller.

The independent audit read the core result: 150 PASS, zero failures, zero ignored, in 0.08 seconds.
The separate all-target Clippy log reaches successful completion.
Host paths and SHA-256 values:

| Evidence | SHA-256 |
| --- | --- |
| `/var/tmp/rom-010-work-production-core150-host.log` | `3df758fa932982580be2d6ef09ee64580675ce66937448019275bc5419ca94a3` |
| `/var/tmp/rom-010-work-production-alltarget-clippy-host.log` | `d3948e0cf9cd0cc29c2a018f5b15adfb7cc4a92ffca01bc5642066a54cabe3c1` |

No blocker remains from the reviewed support corrections.
Native adapter call order, atomic application, format conversion, latency acceptance, full verification, and matching package proof remain separate gates.
This checkpoint does not assert a frozen final source closure.
