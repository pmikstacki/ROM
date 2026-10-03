---
name: rom-resource
description: Use when adding a ROM Resource, custom Field, or revision-checked action to a native Rust application.
---

# Author a Resource and action

Read the [native contract](../../../docs/native-extensions.md) before declaring versions or codecs.
Use the [bundle procedure](../README.md) to select the supplied source checkout and run `resource` preflight.

1. Start from the included [public example](../assets/resource/project/src/lib.rs).
2. Put declarations, codecs, actions, and tests in named modules.
3. Import the public traits that supply the methods you call.
4. Register the Resources and action through the ordinary Builder.
5. Define canonical codec cases with separate input, encoded output, and typed expectations.
6. Test rejected input before implementing its behavior.
7. Execute the revision-checked action with an explicit idempotency key.
8. Test exact replay and changed input under that same identity.
9. Run the included example with the `resource --run` verifier command.

Completion requires executed canonical, rejection, replay, and unchanged-state assertions through public interfaces.
Record the tested source and lock identity from the verifier output.
The example uses a trusted local fixture actor; application authorization still needs an explicit host policy.
Use Resource-owned versions as the contract guide specifies.
