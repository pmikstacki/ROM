# Task 6 native regression gap map

Date: 2026-10-03. Read-only source mapping before author tests.

| Requirement | Existing coverage | Missing direct journey and proposed test |
| --- | --- | --- |
| Managed identity revocation during accepted work | `rom-identity/tests/integration.rs` checks a User change during an action, with SQLite. OIDC cases check current Provider/User/link revisions. Operator verifier races run both stores with synthetic grants. | Use sealed OIDC proofs and the real IdentityGate on both stores. Pause accepted actions, change Provider/User/link, release the proposal and deny commit, replay, metadata and live disclosure. Cancel one caller while ownership remains. |
| Reentrant query policy | `query_read.rs` checks callback ordering and native candidate revocation through wrappers. It does not read native Storage from the policy itself. | Query and row/actor policies call native `Storage::load`. Test reference and uniform eligibility on both stores. Use a bounded child process so a lock regression cannot hang the suite. SQLite selects actual native candidates; redb retains reference fallback. |
| Provider maintenance ordering | Synthetic provider host tests check enabled fields and separate command ownership. Migration runtime tests check historical receipt replay. | Explicitly reject offline migration while Runtime owns the source. Close ownership, preserve identity Resources through migration, reopen and establish new sealed proof before historical replay. This is mechanism evidence; actual provider/browser acceptance remains a host journey. |
| Operator obligation through native migration | Schema migration seeds AtLeastOnce work. Retention migration covers epoch work. Archive/restore/retention tests cover saved operator receipts separately. | Seed ReconcileBeforeRetry delivery, save provider evidence/control receipt, hold a later unknown attempt, then migrate both stores. Verify identity, revisions/fencing, profile, budgets and exact receipt replay without mutation. |
| Active process authentication/upload interruption | Component drain tests and readiness signal tests exist. | Deferred to the maintained Studio host and real provider/blob process fixtures. Native component tests must not be reported as this acceptance. |

No production defect was inferred from this map. Added tests may pass existing behavior.
The new unpublished persistence harness shares the existing signing and bounded subprocess helpers.
The coordinator owns its dependency/lock changes. Core fixes require a reproduced failure and explicit file ownership first.
