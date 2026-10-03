# Independent query selector review

Reviewed source: **93d080510eaffde6c104e41451fccf5bb5abb1f9** on
`codex/prototype-query-selector`, against the standalone selector plan. The
underlying query fixture is `b7cfdb1`. Review completed 2026-10-03. No maintained
ROM query API or production adapter change is approved by this experiment.

**Result: no remaining blocking findings in the reviewed scope.** The seven
selector tests passed. A separate native harness passed 168 query comparisons and
fault checks. The review made no prototype source edits; the author fixed the
reported issues before the final checks. The independent harness is preserved at
[evidence/query-selector-review.rs](evidence/query-selector-review.rs).

## Evidence and resolved findings

| Concern | Initial observation | Final behavior verified |
|---|---|---|
| Cost-dependent budget admission | For 1,000 rows, amount >= 990, descending amount, limit 1, budget 10, fresh statistics selected native execution and returned amount 990. A stale estimate fell back to a full snapshot and failed its candidate budget. | Both strategies first enforce the same actual whole-collection row count, independently of cost statistics. Fresh and stale estimates now both return `collection budget exhausted`. |
| Unknown EXPLAIN shapes | A recognized access node anywhere in the detail list was sufficient; unrelated unknown nodes and index names sharing a prefix were not rejected. | Strict known-node classification rejects unknown details and similar index names. SQLite version is checked against the pinned 3.53.2 fixture. |
| Plan inspected versus plan executed | EXPLAIN omitted the bound LIMIT used by native execution. | EXPLAIN uses the same LIMIT-bearing SQL shape and bound budget as execution. SQLite retains physical plan ownership; no forced-index hint is added. |
| Stale statistics after mutation | The test supplied a new caller epoch; mutation itself did not invalidate estimates. | Wrapped SQLite mutation increments its actual session generation after commit. Prior statistics fall back, and the current committed row is returned. |

The seven repository tests cover query/epoch binding, optional estimates,
unsupported candidates, overflowing cost exclusion, strict EXPLAIN classification,
common budget admission, exact SQLite results with/without the secondary index,
stale and incorrect statistics, and redb's reference path.

The independent harness compares **complete returned rows**, not just IDs, for
2 SQLite index profiles × 7 amount thresholds × 4 orderings × 3 additional-filter
forms = **168 vectors**. Thresholds include zero, boundaries, an empty range and
`u64::MAX`; orderings include no explicit ordering, ascending amount, mixed amount
and title order, and descending note order. Filters include a title requiring
normalization and explicit null notes. Expected results use the normalized plan
and the existing exact oracle, including fixture visibility.

Separate fault-adapter checks establish that denied sort authorization happens
before any adapter call, an optional planning failure selects the reference path,
and a native execution failure returns an error without retrying a snapshot.
A real wrapped mutation proves prior estimates become stale and the fallback
observes the updated `u64::MAX` value. Fresh/stale estimates are also compared at
the formerly inconsistent collection bound.

## Reproduce independently

Run from a checkout containing this report with native Rust 1.99 and the prototype
commit available locally. This creates a separate temporary worktree/package; it
does not modify maintained source. The prototype uses its existing pinned lockfile.
The harness has no dependency except that local prototype.

```sh
selector_review_dir=$(mktemp -d /var/tmp/rom-selector-independent.XXXXXX)
git worktree add --detach "$selector_review_dir/fixture" 93d080510eaffde6c104e41451fccf5bb5abb1f9
mkdir -p "$selector_review_dir/harness/src"
cp docs/research/evidence/query-selector-review.rs "$selector_review_dir/harness/src/main.rs"
cat > "$selector_review_dir/harness/Cargo.toml" <<EOF_MANIFEST
[package]
name = "selector-independent-review"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
rom-query-planning-prototype = { path = "$selector_review_dir/fixture/prototypes/query-planning" }
[profile.dev]
debug = 0
EOF_MANIFEST
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/var/tmp/rom-selector-review-target}"
cargo test --locked --offline --manifest-path "$selector_review_dir/fixture/prototypes/query-planning/Cargo.toml" --test selector
cargo run --offline --manifest-path "$selector_review_dir/harness/Cargo.toml"
```

`--offline` assumes the pinned dependencies are already cached. The recorded run
used the native `rom-dev` environment, two build jobs, debug information disabled,
and `/var/tmp/rom-selector-review-target`, separate from the author's benchmark
target. Expected independent harness output:

```text
fresh and stale reject identically at common collection bound
168 varied normalized query comparisons passed
authorization precedes adapter; planning failure falls back; execution failure does not; mutation invalidates estimates
```

The original scratch harness/logs were at `/var/tmp/selector-review` inside
`rom-dev`; they are not required for reproduction. The checked-in source above is
the durable evidence artifact.

## Limits of the conclusion

- This is a fixed-schema, synchronous prototype. Sort authorization and persisted
  row visibility are synthetic fixtures, not proofs of maintained ROM authority,
  field policy, tenant isolation or changing authorization during execution.
- SQLite generations are connection-session local. Statistics must not be reused
  across reopen. External writers and cross-process statistics invalidation are
  outside this fixture; a maintained design needs coherent admission and reads.
- Query equality and declared exactness bind estimates to the normalized plan;
  the trusted adapter remains responsible for exact native semantics. Costs are
  ranking hints, not evidence of semantic support or authorization.
- Whole-collection admission deliberately retains the bounded-kind profile.
  Selective queries over an oversized kind are rejected under every strategy.
  Candidate bounds do not independently bound database internal scan/sort work.
- Reported VM steps and candidates describe **execution only**. Selected-path
  elapsed time also includes normalization, authorization, count admission,
  EXPLAIN and selection. Raw core/native timing baselines omit that selector
  overhead. These counters are not total end-to-end query work.
- The estimator uses rough distribution and operation weights. Wrong statistics
  can worsen ranking while admitted queries retain exact results. Neither this
  review nor the sampled timings establishes a universally optimal strategy.
