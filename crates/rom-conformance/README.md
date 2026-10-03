# ROM native conformance

Development-only public assertions for native alpha profile revision 1.
Use this crate as a development dependency.
Default features cover Field codecs and storage; feature `blob` adds blob assertions.

`field::codec` checks explicit input, canonical output, expected typed value, and rejected input.
`storage::basic` accepts a fresh, exclusive `StorageFixture` factory.
Its facts are trusted test inspection data; counts mean rows, events, receipts, and effects.
It releases storage clones and Runtime ownership before fixture reopen.
`storage::for_profile` rejects an unsupported assertion profile before fixture operations.
`storage::assert_bundle` shares the fresh-store, two-commit baseline checks with native fault tests.
It requires one Row, two events, two receipts, and two update effects.

`blob::basic` requires a disposable namespace and a sixteen-byte adapter limit.
It deletes its fixed test keys. Real provider acceptance remains an explicit separate test.

Errors expose static case and category metadata without retaining adapter errors or stored values.
Passing these assertions does not replace registration, authorization, crash, migration, or provider recovery tests.
The canonical source guide is `docs/native-extensions.md` in the supplied ROM source tree.
This profile keeps Resource-owned codecs. It provides no independent Field registry or stable binary ABI.
Publication remains disabled.
