# Core and CLI independent review

Scope: the revised follow-on goal, after experimental MVP `54378c9`: deferred capability research, API ergonomics/resilience and a generic CLI. Studio implementation is excluded. This review is separate from the [original MVP review](maintained-mvp-independent-review.md).

## Reviewed core and research

At `a293e0f`, an independent reviewer examined typed `Query::all`/`Default`, semaphore configuration and the complete 37-topic research inventory. Five focused tests passed in the review checkout using Rust 1.99.0 and the locked graph. No substantiated P1/P2 finding remained. Research review checked coverage and sampled primary-source claims; it did not execute the proposed future probes or certify external providers.

Core discovery source commit `b05cabf` was reviewed independently: all 11 discovery tests passed. The reviewer also constructed a multi-kind, multi-field, multi-action catalog and tested every byte budget through its exact serialized size plus one. Smaller budgets returned `TooLarge`, while exact/greater bounds returned the full catalog. That regression was integrated as `4fab19d`. Review covered current authority, revocation, panic supervision, hidden nested references, allocation bounds, deterministic output and the absence of row/codec/action execution during metadata discovery.

HTTP/demo source commit `e858d7e` passed three focused tests in the independent checkout. The route uses the existing authenticated bounded body path; only intended demo metadata is disclosed. Documentation and implementation agreed. The separate full core-branch `scripts/check` also passed; combined CLI release checks are recorded separately after integration.

The CLI selection research was independently checked against primary documentation for clap, lexopt, argh, reqwest retry policy and SSE parsing. It distinguishes available mechanisms, engineering choices and acceptance criteria from executed evidence.

## Findings during CLI implementation

1. **A raw HTTP error cannot prove rollback.** Coordinator review traced arbitrary `ActorGate` errors through observation after `invoke_row` has committed. Even categories such as Conflict or TooLarge can arrive after a durable mutation. The accepted correction treats every unsuccessful response after submission as unresolved; local preflight errors and read/stream rejection remain distinct. This avoids changing core policy semantics or inventing a phase-bearing protocol. A real post-commit gate regression is required before closure.
2. **Responses must match the requested Resource identity.** Independent TCP reproduction made the working CLI request `tasks/requested-id` and returned a structurally valid success for `other-kind/other-id`. Both read and mutation incorrectly exited successfully and printed that unrelated view. Contextual response validation must reject it before output: read protocol failure or unresolved mutation. Related collection and cursor scope checks are included in remediation.

These findings concern an in-progress CLI, not the released MVP. Final remediation evidence and exact integrated verification revision will be appended after fixes; this document does not yet declare CLI completion.
