# Independent Task1 signal and host lifecycle review

**Spec: accepted. Quality: accepted. No open findings.** Baseline 60960510d455bee77906e4cb9807778a21f4082c. Nine final frozen/live file hashes and Cargo.lock matched the supplied source manifest. The original eight files are unchanged by the one-file provider test amendment. No implementation edits or repeated test runs were made by this reviewer. Root owns the complete local gate.

## Implementation

The shared host SignalReceiver installs both Unix receivers synchronously. Synthetic and provider serving install it before either readiness output. The stop future only waits on the already-installed receiver. It cannot defer signal registration until after readiness. Unix signal ownership stays in the demo host. Core and native format8/archive6 are unchanged.

The synthetic flow moved from main to a cohesive serving module. Argument validation remains in main. Library root is a facade of declarations and exports; the existing public paths remain and serving::run is additive. No duplicate receiver exists in provider serving. Test cases use one shared signal driver, existing Scratch, bounded native Process, and provider Node process helpers. The Process PID accessor is additive and narrowly permits separately compiled fixtures that do not need it.

Synthetic shutdown awaits BlobService drain before Http closes Runtime. The outer cleanup also closes BlobService/Runtime after setup or serving failures and preserves the first result error. The existing provider seam still closes and drains accepted authentication before Http/Runtime shutdown. An existing paused-authentication test exercises that ordering. No arbitrary blocking Rust callback hard-walltime promise was introduced.

## Process assertions and evidence

Four actual synthetic binary tests cover immediate readiness-time SIGINT/SIGTERM on SQLite/redb. Child readiness/completion have finite bounds; unwind drops/joins the process. Failed Scratch evidence is retained. The tests require successful exit, stopped intake, zero owned work, no Runtime failure, empty stderr, exclusive native reopen, and previously completed attachment content. The before-fix behavioral log contains two genuine SIGTERM signal15 failures; SIGINT passed and is not called a reproduced race. Final four-case log is GREEN.

The amended provider driver preserves sixteen SIGINT attempts per store and adds one SIGTERM attempt per store. Signal selection and attempt count pass as structured process arguments to the same Node helper. Both tests require successful bounded child exit and empty native store after reopening, so serving cannot silently provision. The supplemental source SHA is e203f573e1d16f63eab8c351a59cccde3ea635382741a10928fe440c94e1fd81. The focused amendment log shows both provider tests GREEN, followed by strict all-feature demo Clippy. Formatting evidence is retained in the author log.

Review identified that synthetic process cases do not pause an accepted attachment operation. The final author report now states this correctly and provides a component matrix. Blob paused-publication drain is supported by the existing component test plus inspected host shutdown ordering; it is not mislabeled as an executed active-attachment process case. Provider paused authentication was executed in the covering suite. These qualifications resolve the review observation without inventing extra production scope or changing the core.

The reviewed evidence is implementer execution and reviewer source/log inspection. Default/all-feature demo suites, affected native-helper/demo strict Clippy, focused process tests, fmt and diff checks passed as reported. No independent full verifier is claimed here. Final coordinator gates still determine integration.
