# AI read progress and Wake: independent review

Date: 2026-10-08.

## Outcome

No new blocking defect was found in the reviewed read progress, physical ownership, or Wake transitions.
The reviewer inspected source and preserved native evidence. No Cargo command or native test ran during this review.
Current full verification and final installed-package acceptance remain separate gates.

## Public progress contract

`flow::ReadProgress` exposes only a finite status and ordinal from one through 32.
It does not expose tool arguments, results, call identities, or a dispatch credential.
Pure `RunView::project` derives durable metadata without claiming knowledge of physical ownership.
An unresolved retained read becomes `ActivityUnknown` in that projection.

Host projection checks current owner authority, retained transcript grants, registry compatibility, and the current tool actor.
The trusted tool actor callback can inspect current referenced rows through `AuthorizationRead`.
No read callback or domain action is invoked by the projection.
Wrong owners, revoked tool grants, and revoked referenced-row grants cannot receive a successful host progress snapshot.

The host observes a weak physical lease to distinguish `Active` from `AwaitingRecovery`.
`ReadContext::calculate` retains a strong lease inside the supervised synchronous calculation closure.
A caller timeout therefore does not remove physical ownership while that descendant still executes.
Read recovery rejects takeover while the lease is active.
After physical completion, an unresolved read can become `AwaitingRecovery`.
After reopening, the new host does not infer physical activity from persisted metadata alone.

These snapshots are advisory. They do not grant retry authority or require an operator identity.
Dispatch and explicit recovery independently check current authority and durable state.

## Wake and accounting

Wake requires a scheduled read with the same call and ordinal.
It retains the scheduled read's original operation key and pending operation.
It adds one bounded explicit request stamp and one tick.
It does not increment the callback ordinal, provider attempt count, checkpoint, or completed tool count.

The host requires current owner, transcript, tool, and referenced-row authority before admission.
It checks current revision and rejects a live physical lease.
The durable transition validates its original scheduled operation and bounded operation list.
Replaying the same explicit operation key verifies requester, kind, and original expected revision.
Execution rechecks current grants before invoking a callback.

Result checkpoint tests cover committed lost acknowledgements and definite noncommit on both SQLite and redb.
Separate tests cover admission acknowledgement loss and interruption before callback execution.
Their oracles retain the original provider attempt, accounting, and callback identity.
Mutating Actions remain on receipt-based outcome reconciliation. Read retry does not authorize repeating an uncertain Action.

## Timing limits

Progress authorization uses a shared two-second execution deadline for its awaited policy and row-acquisition work.
The explicit resume path shares a two-second acquisition deadline and has an outer 18-second timeout.
The initial actor establishment and Resource read in `FlowClient::view` precede the progress deadline.
Therefore, this review does not claim a two-second wall-clock ceiling for the entire view API.
Arbitrary synchronous host policy code must remain bounded; async timeout cannot forcibly interrupt that code.

## Independently checked recorded tests

The reviewer summed actual `test result: ok` lines rather than treating filenames as test counts.

| Preserved log | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| `rom-ai-task5-read-progress-ai165-regressions.log` | 165 | 0 | 0 |
| `rom-ai-task5-read-progress-adapter44-regressions.log` | 44 | 0 | 0 |
| `rom-ai-task5-read-progress-external17-regressions.log` | 17 | 0 | 0 |

The AI log includes both physical CPU timeout tests and both referenced-row grant revocation tests.
It also includes both advisory recovery and queued snapshot tests.
The physical fixture checks Active after the caller deadline, then permits one retry after actual descendant release.
Its pure projection remains ActivityUnknown during that interval.

Scoped AI/adapter and external consumer Clippy logs end with successful completion.
The public compile fixture reports that read/query methods compile and `execute` is rejected with E0599.
These results do not establish a final release-package consumer run.

| Evidence in `/var/tmp/` | SHA-256 |
| --- | --- |
| AI 165 regressions | `c2100c0122b6c4140fc2e55ae3990c493ef56f0c49b1da609ede5d633e34ec20` |
| Adapter 44 regressions | `119dbfd30c43f8a16c82a589cdf6f6521abc36f3289695e3b12542339d70bcfc` |
| External 17 regressions | `6bdd99f5256d9375314e1e80a3d26a83c350f4aa66075f9533acd09a85f21150` |
| AI/adapter Clippy | `c0101961fc4da4ec858d3567e990d740e1ec8b86d364c22273db8caea866d0c3` |
| External Clippy | `a6237faea91627d82940f16d0c1924ca9af81e1436b1956c6ab10ac2494eca76` |
| Public compile fixture | `5f53724f52a33cdc59b5580ce910ce9a9270db2998bbcc9c0a583a168e73d1cf` |

## Reviewed source identity

| Source under `crates/rom-ai/src/` | SHA-256 |
| --- | --- |
| `tools/read_progress.rs` | `1a08b44dbfc1d854c4ce9c3fa0ab03153b90f41540673b47420f3aff98f1fcb5` |
| `tools/read_resume.rs` | `54dd6c240203aec00f2be8677e60915fb8a963a8eb5ea156825e66ff08e0d38e` |
| `tools/read_checkpoint.rs` | `59bd0ed8cfb48fca3d66ff6235f256c3ca70438728207b9cc725498e881b784f` |
| `tools/attempt.rs` | `f068d26e92f38c2d51b608d5df53d1867eef70eceeea5b4ee4ef7405393533eb` |
| `tools/calculation.rs` | `a57d7a709b0f428acbdc17ecc549fa56282a7c3bc2586b95625bfcdc819f221b` |
| `tools/worker.rs` | `7a03e2f18b6a591bea43f2d090998e5c7f846d90cd65d347ba76e64de2eaa925` |
| `flow/projection.rs` | `3d2fe41c82cc45244bb57047ce1ada261d1f213dcfe5dbacd217d872707957f8` |
| `flow/view.rs` | `a2576d78dea285db70e86846852e1280a3c95b633a740712720967c29f869b12` |
| `flow/client.rs` | `da0811ae26347201a9021cef3436a1b48bb0c06e0b6bd22b5f8bfe24170edd3b` |

This table identifies reviewed current source. It does not substitute for a complete compilation-input witness.
Keep earlier failures and corrections as historical evidence. Do not treat this scoped review as final 0.1.0 release admission.
