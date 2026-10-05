# ROM 0.0.3 independent standards review

## Executive result

No hard violation of the documented module-boundary or DRY rules was found in the reviewed changes.
The baseline is `6d3b6b2`, including tracked changes and new untracked files.
This is a source review. It did not run builds or tests.

The review covered `rom-fields`, native enum-label admission, derive presentation, semantic rendering, App lifecycle, reference lookup, and demo declarations.
The reviewer implemented sortable controls, draft helpers, frontend enum metadata, and related tests earlier in this task.
Those changes are excluded from independent judgment.
The later FieldHost lifecycle fix is also excluded.

Two low-priority design concerns remain.
The demo repeats enum wire tokens across shape, labels, encode, and decode.
The application controller also formats reference candidates inside its session orchestration function.
Neither concern demonstrates a current behavior defect.
Both have bounded optional fixes below.

## Documented rules

The controlling rules are [AGENTS.md](../../AGENTS.md) and [quality gates](../quality.md).
Rust facades contain declarations, exports, and cross-module definitions.
Implementation and tests reside in named modules.
Proc-macro entry points retain the documented crate-root exception.

`crates/rom-fields/src/lib.rs:4` declares five focused modules and exports their public field types.
Its string-field implementation resides in `scalar.rs:2`.
That macro shares constructor, decoder, encoder, and codec-identity behavior while each parser retains its own guarantees.
`crates/rom/src/resource.rs:4` declares and exports the enum-label module.
Its tests reside in `resource/enum_labels_tests.rs`.
`crates/rom-derive/src/lib.rs:9` delegates required proc-macro entry points to expansion.
Presentation parsing and generation reside in `presentation.rs`.
The demo facade adds only module declarations.
These changes preserve the existing public paths through exports.

Native label admission is shared between Resource fields and action input descriptors through `resource/enum_labels.rs:13`.
Compile-time presentation diagnostics and runtime registration validation have different guarantees.
Native codec authority and browser input validation also have different guarantees.
Their shared wire vectors are preferable to a generic abstraction across those boundaries.
Their separate implementations are not, by themselves, a documented DRY violation.

## Heuristic concerns

| Concern | Exact source | Concrete impact | Bounded optional fix |
| --- | --- | --- | --- |
| Repeated wire-token literals | `demo/src/studio_choices.rs:14`, `:21`, `:31`, `:39` | A canonical member rename requires coordinated edits to four representations. | Introduce three local token constants. Keep the explicit manual Field implementation and its public-authoring example. |
| Candidate shaping inside orchestration | `studio/src/lib/application/controller.ts:486` through `:503` | Picker admission, title mapping, and local search increase the controller's responsibilities. | Extract a pure bounded candidate helper into the existing reference lookup module. Keep epoch, cancellation, request ownership, and authorization handling in the controller. |

The repeated kind branches in `SemanticField.svelte:108` and its input attributes are currently a small fixed protocol.
The validation switch remains in `semantic-fields.ts:101`, outside the component.
A factory or plugin hierarchy would add speculative abstraction without removing current duplicated behavior.
No required refactor is recommended for those branches.

## Scope and evidence limits

The review read complete focused source modules and compared changed tracked files with the baseline.
It inspected facade exports, registration admission, generated presentation hooks, session guards, lookup limits, and generic demo declarations.
The review does not certify compatibility, security, accessibility, or human authoring usability.
The release task owns executed verification and the full local verifier.
