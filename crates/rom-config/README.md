# rom-config

Bounded JSON/TOML input for existing ROM Resource definitions. The adapter uses
config 0.15.27 with only the `json` and `toml` features. It does not register kinds,
discover files, read process environment variables, merge source layers or write
storage directly.

The parser accepts at most 64 KiB and 128 top-level fields. Duplicate JSON object
keys are rejected recursively, TOML non-finite numbers are rejected, and literal
field names are preserved by collecting the source directly instead of sending
keys through config-rs's path merge API. Omitted values and explicit JSON null
remain different. Resource codecs determine whether a candidate is acceptable.

```rust
use rom_config::{parse, Format};
let candidate = parse(Format::Json, r#"{"enabled":false}"#, "deployment").unwrap();
assert_eq!(candidate.values()["enabled"], false);
assert!(parse(Format::Json, r#"{"enabled":false,"enabled":true}"#, "deployment").is_err());
```

`ParsedDocument` retains host-provided safe origin labels per field and omits
values from Debug. Errors have fixed categories and contain no raw parser
messages, document values or filesystem paths. Origin labels are identifiers,
not arbitrary paths. Resource ids and source permissions live outside documents.

Register `SourceActivation` as an ordinary Resource with `REQUEST_RELOAD` and
explicit host policies. Only trusted administrators may enable a source or change
its target. Requesting or resuming work requires the record's explicit
`worker_authority`/`worker_subject` binding;
being permitted to inspect source configuration does not grant worker authority.
The worker may change `requested_generation`, `source_version`, `target_revision` and `target_present`
through the request action. Declare `.source_owner("deployment")` on the target's
accepted definition and provide separate `.source_metadata_policy(...)` for
protected inspection. Source identity/scope is not a field in the input document.

`ReloadTicket::request` runs before fetch/parse. It persists the next requested
generation and original target revision/lifecycle mode through an ordinary action.
`load` parses complete values, and `apply` calls `Runtime::invoke_sourced`, which
shares normal validation, authorization, field policy, conditional revision,
receipt and bundle execution. `delete` is explicit. Normal mutation entrypoints
cannot overwrite externally owned kinds, and a source cannot write a kind whose
accepted definition does not name it as owner. Permits also restrict the exact
target and require a current generic activation-Resource revision.

Requested generation and accepted generation are deliberately separate. A failed
new request invalidates older pending work but leaves the accepted target value
and provenance intact. Source enable/scope changes invalidate old tickets even
after re-enable. Optimistic target revision still arbitrates concurrent edits.
`resume` recovers the persisted current request without allocating another
generation; replay after a lost commit acknowledgment resolves its original receipt.
The source version must identify immutable input: on recovery the host must fetch
the exact same document or receive IdentityMismatch, not silently substitute a
new body under an old version.

The runtime commits protected source identity, safe version label, generation and
field-origin labels in the target Row/receipt atomically. A provenance-only change
is a real Resource revision and journal fact; exact value/provenance equality is
a no-op. Standard typed/projected results and public raw invocation strip internal
metadata. A protected `source_state` inspection distinguishes requested/accepted
generations without exposing secret values or requiring a second active-state write.
Regular mutations preserve protected metadata; source ownership controls whether
they are allowed in the first place.

Dynamic custom-codec error messages are redacted by `apply`/`load` rather than
forwarded to configuration diagnostics. This preserves fixed runtime categories
such as Conflict, Denied and Unknown without retaining rejected values.

Permits expire at an exclusive host-clock boundary. That bounds source write
authority; it is not a TTL on accepted passive Resource data. Deployments needing
effective-value expiry or live external-service activation must add that policy.
This package does not resolve secrets, configure real provider sessions, start
listeners, or claim rollback of remote effects. Use host-controlled secret-store
references, never credentials in input fields, provenance labels or error context.

This first profile supports one complete Resource per reload and whole-Resource
ownership. It does not offer atomic multi-Resource updates, source precedence,
overlay removal, partial patches, environment providers or writeback. Missing
fields follow the accepted Resource codec; JSON null is a value, and missing files
or fetch failures must not be passed off as an empty source or a delete. Failed
attempts remain visible as returned errors and requested-versus-accepted generation;
raw document bodies/errors are not persisted as diagnostics.

`./crates/rom-config/verify` runs parser, native source-permission and integrated
ingestion tests. Settings, User and IdentityProvider fixtures all use the same
Resource path. SQLite and redb tests inject failure after a native write and after
actual commit, reopen storage, and verify that value/provenance remain paired and
the original receipt replays without another target event. No external account,
network change or production identity-provider deployment is involved.
