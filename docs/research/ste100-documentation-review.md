# STE100 documentation review

Date: 2026-10-03. Baseline: `2988891482fbc749af5887eddf0b9b65460d113d`.

This change applies STE guidance to ROM's technical prose. It does not change runtime behavior, API contracts, or the status of release tasks.

## Skill and scope

The selected [ASD-STE100 skill](https://github.com/nuelcyoung/asd-ste100/tree/3fe2ccb51b1a98fb8d40aa88ec448575b8439ddb) is version 0.5.0, based on Issue 9.
The global installation is `/root/.codex/skills/asd-ste100`. Its files match the selected source revision, including the reference files and license.

Three parallel agents and the coordinator reviewed 164 existing Markdown documents. Each document received contextual prose edits.

| Document group | Documents | Work |
| --- | ---: | --- |
| Public guides, package READMEs, demo, and infrastructure guides | 22 | Separate instructions, explain conditions, and name the component that performs an operation. |
| OpenSpec and implementation plans | 60 | Shorten descriptions without changing requirements, scenarios, or task states. |
| Research reports | 82 | Clarify findings and recommendations while preserving dated evidence and limits. |

The review excludes third-party notices and the license-supplement README. License text, raw evidence files, source code, and lockfiles remain unchanged.
The new [writing guide](../writing.md) records the method and shared technical terms. The quality guide links to it.

## Meaning review

The edits split long descriptions and separate combined instructions. They retain the source's conditions, uncertainty, and recommendation strength.
Historical proposals remain proposals. Test results remain tied to their original source, environment, and workload.

The coordinator reviewed changed passages about authorization, query limits, idempotency, compensation, backup, and recovery.
That review corrected wording that described runtime authorization as a test. Runtime verification and executed test evidence remain distinct.

## Verification

- The full local `./scripts/check` passed in `rom-dev` with Rust 1.99.0.
- This run covered formatting, Clippy, workspace tests, doctests, rustdoc, compile-failure fixtures, dependency isolation, and auth/identity verification.
- Strict OpenSpec validation passed all nine changes after the coordinator's final specification edits.
- `git diff --check` passed.
- The baseline comparison covered all 164 documents and preserved all 94 fenced code blocks byte-for-byte.
- The comparison also preserved headings, original link destinations, API identifiers, numeric evidence, normative keywords, and task states, subject to the additions below.

The full-check log is `/var/tmp/rom-ste100-check.log` inside `rom-dev`. Historical experiments were not rerun to refresh their findings.

The review accepted these differences:

| Difference | Reason |
| --- | --- |
| One additional inline `serve` in the demo guide | It replaces an ambiguous pronoun with the existing command name. |
| A link from the quality guide to `writing.md` | It makes the new authoring guidance discoverable. |
| Spaces in `to3`, `most32`, and `limit1..snapshot_rows` | The original numeric values remain unchanged. |
| Four numbered stages in the release proposal | The list preserves the original four-stage sequence. |

Numeric comparison excludes numbered-list markers and includes digits attached to words. This avoids treating the spacing repairs as new measurements.
Only one existing table row changed: its retention recommendation was split into sentences. Benchmark tables and their measurements remain unchanged.

Inspect the prose changes from the recorded baseline:

```sh
git diff 2988891482fbc749af5887eddf0b9b65460d113d -- '*.md'
```

## Limits and retained exceptions

This is an STE-guided revision, not a certificate of compliance. It is not an exhaustive part-of-speech ruling for every word occurrence.

Vocabulary was checked against known rulings and high-risk patterns only, not against the official ASD-STE100 Part 2 dictionary. Full compliance requires verification against the official standard.

- Normative `SHALL`, `MUST`, `SHOULD`, and `MAY` retain their original force. Advisory language remains advisory.
- Schema terms such as required field retain their domain meaning. Rust identifiers, protocol names, and established technical terms remain exact.
- Long acceptance statements and technical inventories remain where a shorter form risks changing their scope.
- Descriptive passive voice remains where the source does not identify an actor.
- Headings, quotations, code comments, commands, and evidence labels retain their source form.
- Older plans can describe historical scope. This prose revision does not update those plans to claim current implementation status.

The [official ASD-STE100 standard](https://asd-ste100.org/) remains authoritative. A human writer remains responsible for final approval of the prose.
