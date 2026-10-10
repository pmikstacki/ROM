# Export snapshot independent review

Date: 2026-10-07. Status: bounded source and Node review; no blocking correctness finding in the reviewed capture helper.

The candidate captures whitelisted identity fields before invoking the host clone. It retains a detached value and returns another host clone on every value access. It provides no authority, renderer or file operation. The current-authority and request-ownership checks remain separate host obligations.

## Scope and source fence

Reviewed files were [export-snapshot.ts](/root/ROM/studio/src/lib/ui/export-snapshot.ts:1), [ui.ts](/root/ROM/studio/src/ui.ts:1), and [ui-export-snapshot.test.ts](/root/ROM/studio/tests/unit/ui-export-snapshot.test.ts:1). The [primary-source research](rom-0.1.0-ui-latest-primary-research.md) supplies the exact-cloning and disclosure distinctions. Other facade implementations were outside this review.

Evidence is under `/var/tmp/rom-010-ui-export-review/`. `source-before.sha256` and `source-after.sha256` matched across both runs:

| File | SHA-256 |
| --- | --- |
| export-snapshot.ts | `c1ad0e81749fad56885bc52d72fdc0e10ed7e381006c464e6c62eda452d6c176` |
| ui.ts | `49c12d39c410b71e5284a66e51d6b03594d16479416caaf3a42cb659258b4707` |
| ui-export-snapshot.test.ts | `13895e9b9e4c9fe540a155c8bc72d2c5b1b4fcfd6133e086f18911e0ca4adadd` |

No product, repository test or manifest file was changed by this reviewer.

## Executed verification

The maintained command passed five cases:

```sh
node --experimental-strip-types --test studio/tests/unit/ui-export-snapshot.test.ts
```

The independent command passed five additional cases:

```sh
node --experimental-strip-types --test /var/tmp/rom-010-ui-export-review/independent.test.mjs
```

`focused.log` and `independent.log` preserve output. Independent checks covered nested wire edits, renderer mutation followed by retry, principal/resource changes, nested extra-field exclusion, explicit null principal, clone-function replacement and clone failure. No native build, Svelte check, browser or installed consumer was executed by this reviewer.

## Correctness assessment

The helper constructs the complete captured identity before calling `clone(options.value)`. The maintained reentrancy case changed the authority ticket and Resource ID inside that clone. The snapshot retained the original identity. All declared identity fields are copied explicitly. Extra fields on the outer identity, principal and Resource object are excluded.

For the declared primitive field types, the snapshot, captured identity, principal and Resource objects are frozen. Mutating the source principal, Resource revision, selected date or format did not change the capture. The revision remains bigint and the ID remains its exact string. The helper does not serialize identity through ordinary JSON.

The initial clone detaches the source value. Each getter invocation clones that retained copy, so renderer changes cannot alter later attempts when the host clone satisfies its contract. The independent nested wire case changed both the original source and a renderer copy. The next access retained the original large integer and `1e0` member token.

The clone function is captured once. Replacing `options.clone` afterward does not change retries. A clone exception propagates synchronously. Initial failure returns no partial snapshot; getter failure returns no value. This behavior should be handled by the host's computation/error policy.

The generic clone callback is a contract boundary. It must validate its accepted shape, impose the selected bounds, detach nested data and avoid mutating its input. An identity clone of a mutable object or a shallow clone of nested data does not meet that contract. The helper cannot independently prove arbitrary callback behavior.

The exact codec recipe uses bounded `stringifyWire`/`parseWire`. Executed tests cover bigint precision and retained decimal/exponent tokens on container members. They do not establish recovery of lexical information already lost from a primitive Number or ordinary clone.

## Authority and recipe acceptance still required

Captured identity records original context; it does not establish a current grant. A null principal records the host's explicitly public context. It must never reinterpret an unavailable private principal as public data. The helper also assumes the host supplied identity fields of the declared types; it is not an untrusted identity parser.

The host recipe must check current authority and current request ownership after rendering, before exposing bytes, and before a later manual file action. It must check again after any asynchronous or reentrant decision boundary. The value getter itself calls host code, so capture or getter completion cannot replace that ownership check.

A retry retains the captured revision/date/format/locale. A new selection requires a new request. Same-principal authority renewal needs an explicit host decision; principal equality alone cannot refresh a captured ticket. Clearing a UI cannot retract bytes already disclosed.

The helper contains no auth request, renderer, automatic download, file-picker fallback, object URL or persistence. That separation matches the selected module boundary. It also means the five maintained tests do not exercise authority revocation, a held actual renderer, manual-save checks or picker cancellation.

Those actual host/browser journeys remain required by Task 3A. A source import through the facade does not establish package artifact identity, original consumer acknowledgement, release admission or human usability. This review does not close those gates.
