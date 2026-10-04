# Independent Studio discovery review

Date: 2026-10-04.

Scope: the current `studio/src/lib/client/discovery.ts` and `studio/tests/unit/discovery-stream.test.ts` changes. This was a read-only review. It did not modify SDK sources.

## Findings

No blocking issue was found in the reviewed change.

The action-input subset matches native disclosure behavior in `crates/rom/src/discovery.rs`. The core may disclose an authorized action name while withholding its input descriptor because a referenced Resource is hidden. Requiring equal action and descriptor counts rejected a valid response. The client now accepts a subset and still rejects descriptors for an absent action. Duplicate actions and duplicate input descriptors remain rejected. `ResourceDetails` renders forms only for disclosed input descriptors; it does not invent a hidden input schema.

The six new malformed-shape cases match native registration rules in `crates/rom/src/resource/schema.rs` and `crates/rom/src/resource/input_descriptor.rs`:

- Presence (`optional`) is valid only at the top level of an object field.
- Directly nested nullable shapes are invalid.
- Enums require 1 through 256 distinct nonempty values.
- Scalar action inputs cannot describe absence.
- Codec identity names are bounded to 256 UTF-8 bytes.

Existing validation still rejects shape nesting above 16, wrapper paths that disagree with their shapes, unsupported input descriptor versions and zero codec versions. `TextEncoder` counts the codec name in bytes, matching Rust's UTF-8 string length. It does not confuse JavaScript character count with UTF-8 byte count.

The SDK also has its own bounded list and text limits. This review does not claim that every arbitrary Rust descriptor is admitted by those browser limits. Those pre-existing client bounds are unchanged.

## Executed checks

The complete SDK unit suite and Svelte typecheck passed. Exact reviewed source hashes and raw results are retained in [the review evidence](evidence/rom-0.0.2/review-discovery/). These checks do not replace real-host authorization acceptance or the full release verifier.
