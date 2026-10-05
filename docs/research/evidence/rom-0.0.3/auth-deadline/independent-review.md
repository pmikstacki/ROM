# Independent auth deadline review

Date: 2026-10-05.
Scope: the deadline regression in `demo/tests/provider_auth/admission.rs` and the owner's completion contract in `docs/research/rom-0.0.3-release-acceptance.md`.
Result: no material findings in the reviewed final candidate.
This is independent source inspection and evidence inspection, not an independently executed test run or completed release gate.

## Deadline contract

The revised test retains the original `auth` instance, capacity of one job, and 30-millisecond caller response timeout.
It does not reconstruct the tracker or increase its capacity for the recovery assertion.
The provider first acknowledges contact while its response gate remains blocked.
The original caller must return `Error::Overloaded` after its deadline.
A second resolver call before provider release must also return `Error::Overloaded`, and the provider count remains one.

After provider release, the original admitted verifier must drain within one second.
The fresh resolver call uses the same auth instance and tracker.
A bounded observation must see the provider count advance to two.
The admitted verifier must then drain within one second.
The test requires exactly two provider contacts at the end.

The new success condition accepts either the expected Actor or `Error::Overloaded` from the fresh caller.
A successful Actor must retain subject `service` and expiry `NOW + 5`.
`Error::Denied`, `Error::Closed`, and every other error cause a failure.
Allowing `Overloaded` removes the unsupported assumption that fresh verification always finishes inside a 30-millisecond response deadline.
It does not remove evidence of actual admission or completion.

A leaked sole capacity permit cannot pass merely because the fresh caller returns `Overloaded`.
Without admission, no second provider contact occurs, and the bounded contact observation fails.
If job tracking remains active after completion, the bounded drain fails instead.
Premature admission of the pre-release probe cannot pass the complete normal sequence: it introduces an extra contact when the gate opens.
The final exact count detects that additional verification when the fresh retry is also admitted.
These observations exercise the intended ordinary single-capacity fault modes; they do not prove every possible compound tracker defect impossible.

Source inspection confirms the production resolver admits a Job before spawning the detached verifier.
Its spawned task drops the Job after authentication completes and before sending the caller result.
The caller timeout maps to `Overloaded`; a failed response channel maps to `Denied`.
The revised assertion keeps those outcomes distinct.
No production authentication file appears in the tracked diff for this correction.

Reviewed final candidate SHA-256:
`82cec5815f5888a42ca2c6cfcb769a0dcc5f9b72488d55c4bfac2bef315f1ee9`.
The only test change is in the deadline test; the neighbouring cancellation and revision tests are unchanged.

## Evidence inspected

The original instrumented repetition failed at repeat 33 with `Err(Overloaded)` on the fresh verification.
The preserved log is `/var/tmp/rom-003-auth-deadline-investigation/repeat-33.log`.
That failure supports the distinction between successful re-admission and a fast caller response.
It does not alone prove production capacity retention.

The preserved initial revised-candidate `green-repeat` directory contains 40 focused passing runs.
Its saved source, `admission.green.rs`, has SHA-256 `20593b7a0e4ed0a16d35d5f361463ce140f02d80695306c2d032836e893abbe4`.
That candidate preceded the added bound on the original drain; do not attribute those repetitions to the later source hash without separate evidence.

During review, the file owner temporarily inserted a RED control that demanded a second provider contact before the original verifier was released.
The preserved `red-capacity-held-control.log` failed at the intended one-second contact timeout.
The final reviewed source contains no RED control.
This control demonstrates that retained capacity prevents the deliberately premature contact.
It is not a production fault injection or a full capacity-leak mutation test.

No Cargo build or test, production edit, shared port operation, or source restoration was performed by this independent reviewer.
Commands read source, tracked diffs, hashes, and the preserved logs.
The final producer and full verifier remain the release owner's responsibility.

## Owner completion contract

The new contract accurately keeps the Studio and backend requests within one release scope.
It expressly says that mockups and commits do not complete that scope.
It keeps the goal active through complete-artifact independent verification and permanent-preview acceptance.
The document's status line keeps clean-source distribution and persistent preview pending.
Its explicit instruction prohibits treating the record as a completed release declaration.
The later producer failures are preserved and remain outside the accepted evidence set.

No completed-release claim was found in the addition.
The implementation-acceptance table and focused repetitions must still be bound to final clean source, locks, assets, and the release manifest.
The final producer rerun and permanent-preview acceptance are not established by this review.

Reviewed document SHA-256:
`8d385ff49b2d71bedc16433823da2b11f188fc13dfe25a629bb847268a1bbc11`.

## Final source confirmation and new documentation

The release owner confirmed the final source was frozen at the reviewed `82cec581...` digest.
The separate `final-repeat` directory records 40 passing runs for that later bounded-drain candidate.
The repository repetitions manifest identifies those final logs; the older green-repeat evidence remains separate.
The owner reports a provider-auth binary result of 14 passes and two ignored subprocess fixtures.
The reviewer inspected the full local verifier log tail; the owner reported its exit status as zero.
This does not replace an independently executed verifier or complete producer rerun.

The new `docs/research/rom-0.0.3-auth-deadline.md` accurately distinguishes response latency, re-admission, and drain.
It identifies the first 39-pass/one-timeout investigation, the intentionally failing contact control, restored repetitions, unchanged production code, and limits.
The updated acceptance document replaces the ongoing-investigation statement with this bounded proof and still requires final producer acceptance.
Neither document declares the release complete. No material findings were introduced by these documentation updates.

Deadline note SHA-256: `0da9da5b7088bbca09e845b7ed06c5fffecf7fd93f1775a59fbd531729594f4e`.
