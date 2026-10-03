# Scoped archive EPIPE repair review

Date: 2026-10-03. Reviewer: independent of the artifact implementation.
Baseline: `6d59c938a796640b1d64798eb51a8c187b091864`.

**Spec: accepted. Quality: accepted. No findings.**

The correction addresses the observed post-gate broken pipe. `archiveCommit` now gives the existing Git parser only the first 1024 bytes. Complete gzip decompression still succeeds before the parser runs, with the same 30-second timeout and 128-MiB output bound. The parser remains Git's implementation. No custom archive decoder, retry, error suppression, gate bypass, dependency, or format change was added.

The new tests exercise an actual Git archive with an 8-MiB tracked payload, an archive made from a tree without a commit marker, and a gzip stream without its trailer. The latter proves that a readable initial header does not bypass complete gzip validation. Tests reuse the existing fixture and archive helper. Implementation and tests remain in named modules. Public interfaces and the prior profile, selected-inventory, tree, and publication checks remain unchanged.

## Evidence

- Source inspection: the two live files match the frozen hashes below. The existing production delta is the single parser-input change plus its explanation.
- Independent execution: `node --test scripts/release-artifacts/archive-commit.test.mjs` passed all three cases in `rom-dev`. See `independent-review.log`.
- Independent execution: the retained actual source archive returned exactly `6d59c938a796640b1d64798eb51a8c187b091864`. Its SHA-256 before and after the read was `159e88a321a8aa3daae594f6ec9ab96835fbd12cd9bd3c9a98af812887279abd`.
- Author execution inspected: `evidence/green.log` contains 29 passing artifact/shared-skills cases and exact retained-archive commit extraction. `evidence/actual-extraction-green.log` records complete retained source/skills extraction, tree/profile/selected identity checks, and four admission preflights. It explicitly records `examples_executed: false`.
- Coordinator execution reported: 27 artifact/package cases passed. No broad gate was repeated by this reviewer.

| File | SHA-256 |
| --- | --- |
| `scripts/release-artifacts/archives.mjs` | `7115d06ea5c0389614b96dc5b2c72b4f5b3fccc8b4c4478a759666a3033f424c` |
| `scripts/release-artifacts/archive-commit.test.mjs` | `d388c9b5750402731241ea2588878a8b7efba90642790e421f077820c43474ff` |

The first producer remains an incomplete run: all six gates passed, then archive verification failed with EPIPE. This scoped acceptance permits a fresh producer run from the integrated clean source. It does not establish a completed release artifact. The original failed stage and its evidence remain preserved.
