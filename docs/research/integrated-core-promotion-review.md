# Integrated probe: independent promotion review

Reviewed source: `86780041a9824c434054fae25509dff0e56a7212`,
`codex/prototype-integrated-core`, 2026-10-02. A reviewer separate from the implementer reproduced the findings below through a separate public-API consumer in native rom-dev, Rust/Cargo 1.99. No original repository source was changed. The coordinator separately reran the original 18-test suite successfully. That passing result does not remove these additional defects.

| Priority | Defect and reproduction | Required regression/fix |
| --- | --- | --- |
| P1 | Custom action named `delete` and built-in delete share the durable identity namespace. A unit custom action at revision 1 followed by delete at the same revision/key returns the custom action receipt; the Resource remains present. | Tag mutation variants in identity or reserve names. Exercise both orders and reopen SQLite. Never silently replay a different operation. |
| P2 | `read` checks missing storage rows before global revocation. Revoked actor sees `Denied` for an existing ID and `Missing` for absent ID. | Check global authority before lookup; existing/tombstoned/absent all deny without storage work for a revoked actor. Document row-policy concealment separately. |
| P2 | Nested nullable `Some(None)` encodes to null and decodes to outer `None`. Accepted `Option<Option<bool>>` loses information during normalization. | Reject ambiguous nested nullable shapes at shared registration, or define lossless encoding. Test manual/derived parity and all three nested states. |

Locations in the reviewed source: core/src/lib.rs lines 584–604, 753–767 and
81–94 respectively. Scratch reproduction was `/tmp/rom-promotion-review` inside
rom-dev. It is supplemental reviewer evidence, not a retained release test. Maintained regression tests must be committed with the fixes.

Status: all three findings fixed in maintained foundation source `042c4d1`
(main equivalent `5edbb87`). The coordinator independently ran the complete
verifier from an immutable export: 21 runtime tests, one doctest, five intended
compiler failures, renamed consumer, fmt, Clippy, rustdoc and macro-free core
check passed. New tests exercise both built-in/custom operation orders across
SQLite reopen, revoked reads and rejection of ambiguous nested-nullable
declarations. Broader lifecycle/auth/query/chain gaps remain in
the [probe report](integrated-core-probe-results.md) and the
[implementation plan](../../openspec/changes/integrate-resource-mvp/implementation-plan.md).
