# rom-identity

Optional native identity Resources and current identity checks for ROM hosts.
This package depends on `rom` and the dependency-free proof API of `rom-auth`.
It adds no HTTP, JWT, database driver, or policy engine to its normal dependencies.

`User`, `IdentityProvider` and `IdentityLink` use `#[derive(Resource)]` and ordinary
shared actions, field policies, persistence, revisions and observations. A link
uses a canonical `(authority, principal kind, subject)` Resource id and references
a User. There is no email matching, first-caller administrator, or separate user
repository. `ProviderProfile` is a closed custom field codec: unsupported profiles
are rejected before a configuration mutation commits.

```rust
use rom::Resource;
use rom_identity::{User, IdentityProvider};
let user = User::decode(rom::json!({"enabled":true,"display_name":"Example"})).unwrap();
assert!(user.enabled);
assert!(User::decode(rom::json!({"enabled":"yes","display_name":"Example"})).is_err());
assert!(IdentityProvider::decode(rom::json!({
    "enabled":true,"profile":"unsupported","issuer":"https://issuer.example",
    "audience":"rom","endpoint":null,"credential_ref":null
})).is_err());
```

The host installs `IdentityGate` and explicitly allow-lists any trusted embedded
or local service bootstrap identities. It assigns policies to the three ordinary
Resource definitions. Missing trust configuration denies access. This package
does not choose who may create links, change provider settings or administer
Users. Ordinary profile editing must have only its intended field permissions.

Before verification, the host reads `ProviderActivation` using a currently
authorized host identity. Its `verify` callback receives that exact configuration
and returns a neutral `VerifiedIdentity` from a real adapter. The resulting
`ActivatedIdentity` permanently captures the configuration revision. Its `bind`
method resolves the current explicit link and User. It preserves principal kind
and exclusive proof expiry in Actor.

**Trusted callback obligation:** construct the verifier from the callback's
authority, issuer, audience, profile and configured trusted endpoint/key source.
An immutable cached verifier must be keyed by the activation revision as well as
provider id. Never use an old configured verifier or a proof cached under another
activation in the callback. The callback is trusted native integration code, not
an endpoint supplied by a client. Core cannot inspect the internals of native host
code, just as `Actor::trusted` cannot verify a host's credentials.

After binding, the gate point-loads provider, link and User and compares captured
revisions at runtime authorization checkpoints. Configuration changes, unlinking,
disabling and disable/re-enable invalidate previous actors and activated evidence.
If the provider configuration changes before its callback finishes, the callback
cannot establish an actor. This first milestone conservatively invalidates actors on **any** User
revision change, including display-name edits. It does not claim a security-specific
generation. New proof binding can succeed against current enabled records.

Core executes these checks inside bounded blocking work and the commit gate,
with at most eight point reads and a byte budget. It rechecks before commit and
protected receipt/live delivery, then checks generation before async return.
All managed identity changes must use the owning Runtime: direct database writes
or multiple Runtime owners violate the existing Storage ownership contract.
`linked_user_id` exposes only selected metadata for policy code after gate checks;
it is not an authentication function.

Secret values are not identity fields. `credential_ref` names a host secret-store
reference; the host owns resolution, rotation and redaction. There is no remote
OIDC setup, universal provider compatibility, tenant policy, role administration,
password storage or login UI here. Resource/authority namespaces are global in
this milestone. Before a deployment advertises multi-tenancy, it must define tenant membership
and isolation. It must also select its own first-admin provisioning procedure.

Run `./crates/rom-identity/verify` from a Rust 1.99 environment with OpenSSL available
for synthetic test keys. Five integration cases cover actual signed JWT proofs,
ordinary identity Resource actions, explicit links and bootstrap, expiry,
User/provider disablement, relinking, current receipt/live checks, and a User
disable racing an in-flight action. Private keys and tokens stay in memory and
are never logged. Real introspection verification remains covered by rom-auth's
loopback fixtures; this package's mapping is provider-neutral.
