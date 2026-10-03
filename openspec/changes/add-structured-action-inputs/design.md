# Design

`#[derive(Clone, Input)]` implements the existing Input trait directly. Each named member implements Field. `#[input(rename = "wire-name")]` selects the single encode/decode key; `#[input(crate = "::framework")]` supports renamed facade dependencies. There is no Resource identity or registration.

Resource and Input share expansion of the named field list and object codec. Input member decoding uses a hidden facade helper to validate each Field shape. It then applies Field::decode or Field::decode_missing. Presence therefore describes omission in the containing object; Option describes explicit null. The standalone Presence input envelope remains unchanged. Consuming Value::Object avoids cloning the entire input map.

Reject unknown members, absent mandatory fields, invalid values, duplicate/empty wire names, duplicate attributes, generics, and independent Serde configuration. Diagnostics should identify source field types/attributes. Direct codec errors identify the public wire field, not its private Rust spelling. Input::field_names provides a static allowlist generated from the same member list. The runtime preserves only Invalid errors whose kind is input and whose field exactly matches this list. It reports the Resource kind and action.wire-field. Unknown/custom error strings are sanitized to the action name. Scalar and manual inputs default to an empty allowlist.

Action payloads are not nested Resources or persisted independently. Structured Input is not automatically a Field. Nested structured inputs and payload discovery schemas remain deferred to avoid a second schema model. Duplicate JSON keys cannot be recovered after deserialization to Value; raw transport parsers own that boundary.

Validation: public consumer action/replay, omission/null/custom codec tests; negative compile cases with source spans; actual renamed Cargo dependency; existing full workspace verification.

## Executed evidence

- Initial public consumer failed compilation because Input derive did not exist; six final consumer tests pass, including actual action invocation/replay and invalid input producing no commits.
- Ten compile-fail fixtures verify primary source spans (five new Input cases), and actual Cargo dependency alias plus existing Resource fixtures pass.
- A regression test first reproduced the raw Resource identifier wire-name change, then passed after preserving the preexisting Resource spelling. New Input objects use the ordinary raw identifier spelling without `r#`.
- Full `./scripts/check` passed on Rust/Cargo 1.99.0 with debug symbols disabled, using `/var/tmp/rom-release-inputs-target`. Evidence log: `/var/tmp/rom-release-inputs-check-final.log` inside rom-dev. This includes strict OpenSpec, Clippy, workspace tests, rustdoc, no-default-feature core, consumer, compile fixtures and auth/identity checks.
- Independent reviewer reran six consumer tests, ten diagnostic fixtures, actual dependency alias and seven external codec probes; both discovered compatibility/diagnostic issues were corrected before completion.
