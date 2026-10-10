## 1. Neutral contracts

- [x] 1.1 Add optional crate and bounded provider/request/result/error contracts.
- [x] 1.2 Implement run-local pure policy/cursor decisions and exact cost parsing.
- [x] 1.3 Verify independent runs, capabilities, backwards/expired time and sanitized telemetry.

## 2. Durable state and reservations

- [ ] 2.1 Add strict run/budget Resources and pure revision-checked actions.
- [x] 2.2 Implement conservative account/run reservations with unchanged replay identity.
- [x] 2.3 Verify concurrency, restart, lost usage and partial reservation commits on both adapters.

## 3. Existing Work facade

- [ ] 3.1 Add explicit host install/bind and client submit/view/cancel APIs.
- [ ] 3.2 Compose one bounded tick through durable channels and checkpoints.
- [ ] 3.3 Verify duplicate deliveries, unknown-outcome holds, binding lifetime and nested admission.

## 4. OpenRouter adapter

- [ ] 4.1 Review existing dependency graph, primary MSRV/license/advisory evidence and exact features.
- [ ] 4.2 Implement bounded catalog/transport/schema/tool/error adaptation.
- [ ] 4.3 Verify deadline, endpoint isolation, native tool calls and unresolved missing-ID reconciliation offline.

## 5. Tools and cancellation

- [ ] 5.1 Add restricted typed reads and prepared typed Resource action tools.
- [ ] 5.2 Verify current authority, original receipts, bounds, cancellation and redaction.

## 6. Acceptance and integration

- [ ] 6.1 Add two unrelated consumers using public interfaces only.
- [ ] 6.2 Verify staged publication and task triage on SQLite and redb.
- [ ] 6.3 Document measured support limits and author recovery workflow.
- [ ] 6.4 Run affected checks and full local verifier before integration.
