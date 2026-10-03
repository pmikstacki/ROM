# Large archive commit-header fix

Status: source frozen for scoped review on 2026-10-03. No commit or producer rerun was made.
Baseline: `6d59c938a796640b1d64798eb51a8c187b091864`.
Ownership: only `scripts/release-artifacts/archives.mjs` and the new `archive-commit.test.mjs`.

## Observed failure and cause

The actual producer passed all six verification commands, then failed archive verification with `spawnSync git EPIPE`.
Its incomplete stage remains at `/root/ROM/dist/.rom-release-stage-CgCL8D/`.
The source and skills archives, failed manifest, and gate logs were preserved.
The coordinator reported the six-command result; the author independently inspected `failure.json` and reproduced the same error against its source archive.
Evidence: `evidence/actual-archive-red.log`.

Git's [get-tar-commit-id contract](https://git-scm.com/docs/git-get-tar-commit-id) consumes the first 1024 bytes, then exits.
The [pinned Git 2.47.2 implementation](https://raw.githubusercontent.com/git/git/v2.47.2/builtin/get-tar-commit-id.c) reads two 512-byte records.
The helper supplied the whole decompressed tar through synchronous subprocess stdin.
Git closed stdin before the larger write completed, causing the observed broken pipe.
The small original fixtures did not expose this mismatch.

## Narrow correction

The helper now passes `tar.subarray(0, 1024)` to Git.
It still decompresses and validates the complete gzip file within the existing timeout and 128-MiB bound.
Git still parses the header and rejects absent commit markers.
No retry, error suppression, gate bypass, dependency, format, or other module change was introduced.

## Executed evidence

| Check | Result | Evidence |
| --- | --- | --- |
| Actual retained source archive, before fix | Reproduced EPIPE | `evidence/actual-archive-red.log` |
| Real Git fixture with 8-MiB tracked payload, before fix | Behavioral RED with EPIPE | `evidence/large-archive-red.log` |
| Large fixture after fix | Exact expected commit returned | `evidence/green.log` |
| Tree archive with no commit marker | Rejected | `evidence/green.log` |
| Gzip missing its trailer | Rejected; expected gzip diagnostic preserved | `evidence/green.log` |
| Artifact and shared skills suite | 22 artifact plus seven shared skills tests passed | `evidence/green.log` |
| Both owned Node syntax checks | Passed | `evidence/green.log` |
| Owned whitespace | `git diff --check` passed | Executed before freeze |
| Actual retained archives, complete extraction and admission | Exact source/tree/profile/selected identity and all four preflights passed | `evidence/actual-extraction-green.log` |

All author commands used `rom-dev` and the existing worktree.
No Rust compilation, shared target cleanup, or failed-stage modification was performed.
The actual source archive returned the exact original commit `6d59c938a796640b1d64798eb51a8c187b091864`.
The retained extraction test used the unchanged clean main source at that commit; it created and removed only its own verification scratch directory.
This validates the retained archives, not a new complete producer invocation or a completed release directory.

`source/`, `source-sha256.txt`, `changes.patch`, and `untracked.txt` identify the two-file correction.
Scoped review and a fresh complete producer run after clean integration remain coordinator gates.
The six production commands remain mandatory and unchanged.
