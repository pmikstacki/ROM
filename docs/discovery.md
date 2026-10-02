# Authorized Resource discovery

`Runtime::discover(&actor).await` returns metadata that the accepted Resource
definitions explicitly allow that actor to see. It uses the same supervised,
bounded I/O and current-authority checks as observations. A transport must use
this method rather than `Runtime::descriptor<R>`, which remains trusted-host
inspection.

Discovery defaults to an empty catalog. Opt in each item deliberately:

```rust,ignore
Task::definition().discovery_policy(|actor, target| {
    actor.subject == "operator" && match target {
        rom::DiscoveryTarget::Resource => true,
        rom::DiscoveryTarget::Field(name) => matches!(name, "title" | "done"),
        rom::DiscoveryTarget::Action(name) => name == "complete",
    }
})
```

A Resource grant is required before any of its fields or actions are disclosed.
The callback can capture trusted host configuration and must be `Send + Sync`.
Like other native policy callbacks, it must terminate and must not recursively
call into the Runtime. It may run again after a concurrent invalidation. A panic
is supervised and makes the Runtime terminal, rather than returning partial
metadata.

The public types are `Discovery { version, resources }`,
`DiscoveredResource { kind, version, fields, actions }`, and
`DiscoveredField { name, shape }`. Version 1 serializes as:

```json
{"version":1,"resources":[{"kind":"tasks","version":1,"fields":[{"name":"done","shape":{"type":"bool"}},{"name":"title","shape":{"type":"string"}}],"actions":["complete"]}]}
```

Kinds, fields and action names are sorted. Shapes use a snake_case `type` tag:
`string`, `bool`, `u64`, `i64`, `f64`, `nullable`, `optional`, `list`, `map`, `enum`,
and `reference`. Containers carry their nested shape in `value`; enums carry a
string array. References carry `"value":{"kind":"target-kind"}`. A field is
omitted completely if its shape contains a reference to a kind hidden from this
actor, including references nested in containers. There are no hidden totals,
omission markers, source ownership, policy names, or sample/default values.

Metadata visibility grants no permission to read rows, use query predicates, or
invoke mutations. Those operations retain their own current authorization,
including checks against actual prior and proposed values. Discovery does not
scan rows, call field codecs or actions, or evaluate row policies against invented
values. Action names carry no input schema: native `Input` codecs are opaque.

The entire serialized response must fit `Limits::snapshot_bytes`. Construction
charges the envelope and borrowed metadata fragments before copying them into
output. Exhaustion returns `TooLarge`; it never returns a truncated catalog.
Expiry, local revocation, and current identity denial prevent disclosure. Treat
each result as a current observation, not a permanent permission grant.

HTTP clients POST the empty object `{}` to `/discover` under the normal host
credential resolver and body limits. Unknown fields, non-object requests, and
duplicate keys are rejected. The demo publishes only Task and InventoryItem
metadata to its existing domain actors; configuration, identity and attachment
metadata remain undiscoverable by default.
