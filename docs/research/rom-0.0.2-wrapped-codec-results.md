# Custom codecs in container fields

The actual Studio trial found that `Option<CustomField>` lost its codec identity during descriptor generation.
The browser then selected an ordinary editor for a value with a custom codec.

ROM now includes a `codec_wrappers` path in authorized presentation metadata.
The path contains built-in wrappers from the outer container to the custom codec value.
The supported wrappers are `optional`, `nullable`, `list`, and `map`.

An absent or empty path means that the codec owns the complete declared shape.
This preserves the existing contract for custom container codecs.
A nonempty path selects the custom renderer only at the codec value.
The generic container renderer still handles the surrounding optional value, list, or map.

Registration validates the path against the declared shape. The browser independently validates the received path.
Both reject more than 16 wrappers, incompatible shapes, and a nonempty path without a codec identity.
Unknown custom codecs remain read-only. The Studio does not substitute an ordinary editor for them.

The change does not alter the persisted catalog or encoded Resource values.
Manually constructed Rust `FieldCodec` and input descriptor values need the new path member.
Use an empty vector when the custom codec owns the complete value.

## Executed tests

The coordinator used a temporary mutation to remove the optional codec identity.
The regression test then failed with the expected missing identity. The coordinator restored the implementation immediately.
A separate regression demonstrated the missing codec path in `Presence<T>` before its correction.
The initial compile failure is retained as fixture evidence, not as a behavioral regression.

The final native run passed the ROM tests, the three Studio discovery integration tests, and strict Clippy.
The discovery tests verify that presentation metadata leaves the native catalog and values unchanged.
The browser descriptor test rejects malformed paths and accepts nested codec paths.
Actual renderer and native-browser acceptance have separate evidence from the Studio worker.

Evidence:

- [Optional codec behavioral regression](evidence/rom-0.0.2/client/wrapped-codec-behavior-red.log)
- [Optional codec regression stderr](evidence/rom-0.0.2/client/wrapped-codec-behavior-red.stderr.log)
- [Presence codec regression](evidence/rom-0.0.2/client/presence-codec-red.log)
- [Browser descriptor regression](evidence/rom-0.0.2/client/wrapper-discovery-red.log)
- [Final native acceptance](evidence/rom-0.0.2/client/wrapped-codec-native-acceptance.log)

These tests establish the declared wrapper contract. They do not establish behavior for arbitrary third-party renderers.
