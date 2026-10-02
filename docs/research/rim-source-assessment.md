# RIM source assessment and ROM carry-forward

Reviewed 2026-10-02 from the owner-supplied rim.zip, SHA-256 `82c54a7bf2700b73c77ec0223924ca51c16a87cdd57a8181072f5637fee06897`. This is static inspection of the generator itself. No Go program or RIM tests were executed, as requested. Private application code, operational device issues and application-specific mutation bugs are not evidence against this library. Source files are identified relative to the supplied archive; the archive and implementation are not copied into this public repository.

## Scope and premise

The supplied artifact implements parsing, a normalized Resource/field model, schema construction, query helpers and Go/JSON emitters. The owner clarified that this generator was the delivered subset of a broader reactive Resource design after scope reduction. The absence of the later runtime is therefore a scope fact, not a design failure. ROM carries the broader premise into a new Rust implementation.

## What is worth preserving

- One authored declaration feeds both server-side descriptors and client metadata. `parser/types.go`, `generate/generate.go` and both emitters show this concretely.
- A common field capability table drives type-level aggregate/ordering metadata. `capabilities/capabilities.go` also copies returned aggregate slices, avoiding accidental mutation of the shared table.
- Spec/status distinctions, labels, choices and references already support generic UI generation. Secret fields are explicitly excluded from generated filtering, sorting and default columns in `emit/meta_json.go`.
- Schema and generation tests cover useful authored examples, including defaults and metadata. Their presence is inspection evidence; no pass count is claimed here.

## Concrete limitations and their ROM consequences

| Source observation | Consequence / qualification | ROM response |
|---|---|---|
| `parser/parser.go:422` resolves unknown identifiers and most qualified types to string; special imported types are recognized by their spelling. | A custom alias/import can receive metadata that does not describe its actual codec. This is a static implementation finding, not a reproduced deployed failure. | Explicit `Field` implementations; unsupported Rust fields fail compilation, registration validates supported shapes and one accepted codec canonicalizes both mutations and queries. |
| `parser/parser.go:204` reads only the first name of a grouped field declaration and skips embedded fields. | Those forms have narrower coverage than ordinary named fields. | Derive explicitly accepts concrete named fields; unsupported forms receive diagnostics. No silent metadata guessing. |
| `generate/generate.go` projects array item metadata only one level deep; object shape is broad in `schema/schema.go`. | Nested schema detail can be lost between the parsed model and the emitted schema. | Recursive shape/codec contract and bounded nesting, verified for nested lists/maps/nullable values. |
| `parser/parser.go` locates directives by preceding comment position and infers requiredness from pointers/omitempty; form hidden/readonly also sets read-only metadata. | Syntax/layout and presentation hints influence the inferred contract. Optionality, write policy and presentation need explicit distinctions. This does not establish that RIM itself lost false during a database update. | Rust attributes attached to the actual declaration, explicit Presence versus nullable Option, separate write/read/query policy and future Studio renderer metadata. |
| Resource IDs key maps in `generate/generate.go` and `emit/meta_json.go`; parser package construction has no final duplicate-Resource gate. | Later entries can replace earlier JSON descriptors; generated-code behavior is a separate path. | Single registration gate rejects duplicate kinds/fields/actions and inconsistent references before startup. |
| `generate.Run` writes Go output before JSON output; `EmitMetaJSON` writes per-kind files before the registry. | An emitter/filesystem failure can leave outputs from different generations. It is not an atomic publication step. | Derive is compiled with the declaration; runtime source publication uses conditional commit plus provenance. Any future Studio metadata export needs its own atomic generation publication. |
| Secret metadata marks password/writeOnly for string schema and hides certain client affordances. | Metadata is not encryption, authorization or a delivery-time secrecy guarantee. No claim is made about protection in the surrounding private application. | Current row/field policy on disclosure, secret references for provider credentials and protected internal metadata; database-at-rest encryption remains a host/storage deployment concern. |

The most useful architectural improvement is to keep one accepted Resource description through every layer. ROM's maintained tests now exercise the same definition through typed actions, field normalization, conditional persistence, HTTP, live queries and durable reactions. Implementation complexity belongs inside those shared mechanisms.

## Boundaries

This assessment does not claim a complete semantic Go type checker review, a performance comparison or runtime execution of RIM. The referenced historical application chats were not used as proof for these source findings. Their live read did not return during this pass. ROM's executed evidence is recorded separately in the maintained reports and prototype inventory; its tests cannot retrospectively certify RIM.
