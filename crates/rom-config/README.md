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

`ParsedDocument` retains host-provided safe origin labels per field and omits
values from Debug. Errors have fixed categories and contain no raw parser
messages, document values or filesystem paths. Origin labels are identifiers,
not arbitrary paths. Resource ids and source permissions live outside documents.

The current parser tests exercise JSON/TOML type preservation, literal dotted
fields, null, duplicates, malformed input, non-finite TOML, size bounds and
redaction. Runtime ingestion, ownership and durable provenance are the next
implementation step on this branch; parsing alone is not configuration activation.
