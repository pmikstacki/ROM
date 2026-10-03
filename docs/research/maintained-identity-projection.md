# Maintained identity and field protection

This implementation belongs to the maintained Resource runtime. Credential
verification remains separate in `rom-auth`; no provider protocol dependency is
introduced into core.

`Actor::trusted` creates an embedded host identity; `with_kind` explicitly selects
Human or Service. Authority, kind and subject jointly scope revocation and durable
idempotency. Actor has no deserializer. Its optional host stamp is non-secret validation context. Debug omits this stamp, and receipt identity excludes it.

An optional `ActorGate` checks current host identity state through at most eight
point reads, charged against the runtime command byte limit. It receives only a
read-only interface. It runs in tracked bounded blocking work, including commit
and cached-result disclosure checks. Cheap expiry/revocation checks remain safe
on the async thread; the existing generation check prevents delivery after a
managed Resource mutation invalidates an observation. Gate code must not call
back into Runtime. Gate errors deny, and panics terminally fail the runtime.
Without a gate, the host has selected the embedded trusted-actor profile.

Row, field, and predicate permissions are separate and default to deny.
`Definition::allow_all_fields()` is an explicit whole-record field/predicate
grant used by the existing application fixtures; row permission still applies.
Applications handling sensitive fields should supply `field_policy` and
`query_policy` instead. Predicate permission is checked before snapshot reads,
including empty datasets; permitting a predicate is an explicit disclosure of
membership information even if its value is omitted from projection.

`ProjectedView` contains a key, revision, and optional field map, and has no
conversion into a complete typed Resource. Projected invocation/read/query/live
APIs disclose only fields permitted for both current and historical outcome
states. Complete typed and raw-row APIs require every field to be readable.
Consequently a typed action can commit and then return Denied if its full result
cannot be disclosed; callers needing write-without-read use projected invocation.
Cached receipt retries do not execute the transition again.

Create/replace/delete require field permission for all supplied/removed fields.
Custom actions require permission for every changed field, checked against both
prior and proposed state and rechecked at commit. Mixed forbidden changes fail atomically. ROM never silently drops forbidden fields. This first profile offers
only equality queries. Ordering, aggregate counts, cursor queries and arbitrary
expressions remain unsupported.

Tombstone outcomes contain revision/key only under current actor checks; there
is not yet retained row-level tombstone authorization metadata. Journal adapters
must use `project_outcome` under the same observation gate and must not serialize
raw storage receipts or actor stamps.

Evidence: `examples/consumer/tests/actor_gate.rs` exercises kind isolation,
authoritative I/O placement, cached/live denial, bounded reads and panic failure.
`projection.rs` exercises hidden fields, hidden predicates on empty datasets,
default denial, complete typed-read denial, mixed writes, derived changes,
write-without-read, receipt replay, and buffered live revocation.
