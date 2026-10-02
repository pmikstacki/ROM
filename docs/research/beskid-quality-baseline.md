# Beskid quality baseline for ROM

Reviewed 2 October 2026. This is a bounded representative source audit, not a full code, security, performance, or release certification. ROM gates below are proposals, not delivered guarantees.

## Scope and provenance

Public sources inspected at these exact commits:

- [Beskid root](https://github.com/Cyber-Nomad-Collective/beskid/tree/abfe7d6bf1628db1be75245693b699394eb1a23c): `abfe7d6bf1628db1be75245693b699394eb1a23c`.
- [Compiler](https://github.com/Cyber-Nomad-Collective/beskid_compiler/tree/44a07aed5a13d41d57853445167c0f091cd10a7e): `44a07aed5a13d41d57853445167c0f091cd10a7e`, matching the root's compiler gitlink.

Shallow clones were inspected read-only. No Beskid implementation code was copied into ROM. The change-review skill was read, but its fixed-point diff workflow does not apply to this baseline audit: no proposed Beskid change was supplied. Direct source inspection was used; GitNexus query access was unavailable, and no symbols were edited.

The sample covers OpenSpec configuration and a dependency-cycle capability, Linux automation and Rust gate scripts, compiler workspace configuration, operations authorization and tests, persistence exports and tests, authentication boundary tests, and manifest determinism tests.

## Strengths to retain

- **One normative authority.** [OpenSpec configuration](https://github.com/Cyber-Nomad-Collective/beskid/blob/abfe7d6bf1628db1be75245693b699394eb1a23c/openspec/config.yaml) requires observable behavior changes to include specification deltas. Proposals address compatibility, migration, and reversion; designs address security, observability, rollback, and authority boundaries. Retain these principles with a smaller initial capability set.
- **Strict validation.** [The standard gate](https://github.com/Cyber-Nomad-Collective/beskid/blob/abfe7d6bf1628db1be75245693b699394eb1a23c/scripts/ci/woodpecker-standard.sh) checks standard structure, traceability, layouts, and strict OpenSpec validity with explicit tool versions. Structural validity alone does not prove semantic completeness.
- **Explicit Rust checks and bounded execution.** [The Rust gate](https://github.com/Cyber-Nomad-Collective/beskid/blob/abfe7d6bf1628db1be75245693b699394eb1a23c/scripts/ci/compiler-rust-gate.sh) denies Clippy warnings, runs workspace tests, rejects reintroduced legacy patterns, and applies timeouts. It explicitly excludes `beskid_e2e_tests`, so it does not run literally every test.
- **Domain decisions separated from transport.** [Operations authorization](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_pckg_operations/src/authorization.rs) accepts a principal and returns an explicit decision without HTTP concerns. [Authentication tests](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_pckg_auth/tests/verifiers.rs) cover injectable verification, insufficient scope, and hiding private resources from outsiders.
- **Focused persistence boundary and behavioral tests.** [Store exports](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_pckg_store/src/lib.rs) expose contracts over focused modules. [Store tests](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_pckg_store/src/tests.rs) cover uniqueness, invalid metadata, checksum-dependent idempotency, and reversible state transitions. The inspected examples use an in-memory repository; these alone cannot establish database equivalence.
- **Negative and invariance tests.** [Operations tests](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_pckg_operations/src/tests.rs) cover ownership, bounded retention, configuration rejection, and scope precedence. [Manifest tests](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_manifest/tests/abi_v5_source_authority/parser_and_determinism.rs) verify invariance under collection reordering and reject malformed contracts.

## Verified gaps and ROM improvements

### Ordinary Linux pushes do not execute the inspected compiler gate

In [the Linux pipeline](https://github.com/Cyber-Nomad-Collective/beskid/blob/abfe7d6bf1628db1be75245693b699394eb1a23c/.woodpecker/linux.yml), normal pushes run standard validation and shell/JavaScript migration tests, then exit before native build and release gates. [Release gates](https://github.com/Cyber-Nomad-Collective/beskid/blob/abfe7d6bf1628db1be75245693b699394eb1a23c/scripts/ci/woodpecker-release-gates.sh) invoke the compiler Rust gate, but that path is reached on manual build or tag events. Passing this ordinary push workflow therefore does not demonstrate Rust compilation, Clippy, or workspace-test success. This is a finding about the inspected workflow, not every possible external check.

**ROM improvement:** run correctness checks on every pull request and main-branch push from the first executable crate. Packaging is additional. Repository protection must require actual check statuses when available; merely adding a workflow does not enforce merge policy.

### Some migrated scenarios are not concrete conformance examples

The inspected [dependency-cycle capability](https://github.com/Cyber-Nomad-Collective/beskid/blob/abfe7d6bf1628db1be75245693b699394eb1a23c/openspec/specs/compiler--resolution-and-projects--dependency-graph-and-cycle-policy/spec.md) requires cycle reporting, but its scenario merely says exercising the contract satisfies its obligations. It does not supply a graph, operation, or expected diagnostic. This applies to the sampled capability, not every Beskid specification.

**ROM improvement:** scenarios describe observable input, operation, and result. Example: two actions reading revision 7 cannot both commit revision 8; one succeeds and one receives a conflict, with no event for the rejected action. Link scenarios to tests as implementation arrives; distinguish specified from delivered behavior.

### Compiler safety policy should not be copied into a resource core

The [workspace manifest](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/Cargo.toml) allows unsafe Rust and warns on unused must-use results. This can be appropriate for a compiler/runtime and is not a defect allegation.

**ROM improvement:** forbid unsafe Rust in the domain core and deny ignored must-use results. Isolate any later justified unsafe integration with documented invariants. Do not copy compiler-specific architecture or naming conventions merely to imitate quality.

## Proposed measurable ROM gates

| Stage | Gate and evidence |
| --- | --- |
| Specification bootstrap | Strict OpenSpec validation, concrete requirements, unresolved choices explicitly labeled |
| Behavior changes | Spec delta and concrete success/failure scenarios; test links when implemented; compatibility and migration impact |
| First Rust crate onward | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked`, including doctests |
| Public API | Documentation build denying warnings, compiling downstream example, declared minimum Rust version and stable CI checks |
| Extensions | External example plugin using only public contracts; custom field encoding, validation, and error propagation |
| Persistence | Shared adapter contract suite against each supported real database: atomic state/event commit, rollback, conflict, restart persistence |
| Mutation semantics | Tests for missing/null/false/zero distinctions, rejected-action no-op and no-event, monotonic revisions, validation boundaries |
| Reactive delivery | Fault injection around commit/acknowledgement; duplicate delivery, replay, bounded retries and backpressure; documented ordering scope |
| Dependency health | Automated advisory and license-policy checks with reviewed expiring exceptions; MIT-compatible distribution rather than requiring every dependency to be MIT |
| Release | Semver/API review, migration notes, changelog, packaged consumer smoke test |

Test documented feature combinations and the minimal core without HTTP/Rabbit dependencies. Add focused fuzzing or concurrency model-checking when relevant interfaces exist. Numerical performance budgets need measured workloads first. Core contracts use native Rust extensions initially; WASM remains future work. Plugins must not bypass mutation validation, concurrency control, or persistence guarantees.

## Verification and limits

Successfully executed at the pinned root:

- `bash scripts/ci/test/compiler-rust-gate-timeout.test.sh`
- `bash scripts/ci/test/woodpecker-workflow-contract.test.sh`

These fixture-based shell tests verify orchestration, not the compiler itself. No compiler build, database integration suite, full strict OpenSpec validation, dependency/security audit, production inspection, or branch-protection inspection was performed. Reading Rust tests establishes intended coverage, not passing execution.

The evidence supports specific stronger ROM gates, not a numerical quality ranking or a claim that ROM is already higher quality than Beskid.
