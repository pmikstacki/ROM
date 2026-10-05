# ROM 0.0.3 package fixture portability review

Reviewed the uncommitted correction relative to `83f20c2f44c3bc28474b20b7d5863aadd595aca3`.
Source review only. No repository edits or builds were made.

## Result

No source-level spec or quality finding.

- Both moved fixtures are byte-identical to the prior versions. SHA-256 values match: discovery fixture `6d6d9a10cf89b01b2c6c4fe510ea2032ec6e0f212f0f541621b16af172cd9f8e`; invalid fixture `351026ddc22a9518feafcbf877cfd3a2c1537ea40e1f058f89a6a0efebf94668`.
- The positive discovery fixture now sits inside `examples/consumer/tests/fixtures`, beside its `include_str!` in `examples/consumer/tests/enum_labels.rs`. The extracted application copier includes the complete `tests` tree, so this dependency stays inside the copied consumer.
- The negative fixture now sits inside `crates/rom/tests/fixtures`, inside its owning Cargo package. `crates/rom/src/resource/enum_labels_tests.rs` resolves that crate-local path. The manifest has no restrictive `include` list.
- The frontend references the same two files; repository search found no duplicate fixture corpus. Only test/doc paths changed; no public API or runtime behavior changed.
- `scripts/check-packages.mjs` now runs `cargo test --offline --lib` against the extracted `rom` archive. This checks that Cargo includes the core-owned fixture and that the old sibling-crate dependency is gone.
- Documentation links and the new package-acceptance explanation match the moved paths and distinguish the old failed producer from the corrected source. The report says final producer/artifact acceptance remains pending, so the available full-check/frontend logs are not overstated as package-gate success.

The exact archive inclusion and extracted-core test remain for the active package/release gate to prove; this review did not run that gate.
