## 1. Reference application and author ergonomics
- [x] 1.1 Add a runnable public-API recovery journey and actual process-exit tests on both stores.
- [x] 1.2 Audit and improve author friction with concrete compile/runtime evidence.
- [x] 1.3 Expand the reference app to exercise related Resources, policies, live reads and attachments as one documented workflow.
- [x] 1.4 Independently review the application and freeze stage-one evidence.

## 2. Relations and durable lifecycle
- [x] 2.1 Specify and enforce restrict references atomically. Test concurrent create/delete on both stores.
- [x] 2.2 Implement versioned offline migrations preserving receipts and pending obligations; inject interruptions.
- [x] 2.3 Implement dependency-aware retention and identity-expiry behavior with explicit host policies.
- [x] 2.4 Verify upgrade/backup/restore of the reference application and review compatibility.

## 3. Measured execution optimization
- [x] 3.1 Specify the maintained planner-adapter and selector seam using prototype evidence.
- [x] 3.2 Integrate indexed execution with atomic index maintenance and rebuild/recovery.
- [x] 3.3 Compare execution strategies on independent/skewed workloads including writes, allocations and memory.
- [x] 3.4 Prove shared auth/query/admission semantics and independently review integration.

## 4. Operations and release
- [ ] 4.1 Enforce single-writer ownership; exercise overload, process exit and restart recovery.
- [ ] 4.2 Provide CLI work inspection, retry, reconciliation and explicit compensation workflows.
- [ ] 4.3 Deliver a documented real-provider identity/bootstrap/secrets integration profile and tests.
- [ ] 4.4 Version extension contracts. Publish shared conformance fixtures and executable author skills.
- [ ] 4.5 Run the reference application upgrade/recovery acceptance, full local checks and package verification.
- [ ] 4.6 Produce release artifacts, compatibility/support notes and a requirement-by-requirement completion audit.
