# ROM 0.0.2 dependency checks

Recorded: 2026-10-04. These checks identify known advisories at the recorded time. They do not prove absence of vulnerabilities.

The locked native graph contains 282 external Cargo packages. The refreshed source notices retain each package license declaration.
Cargo audit 0.22.2 reported zero matching advisories. Its RustSec database contained 1290 entries at commit `ef6173cbc5c50ec8166f9a5b28f07834144373ee`.
The database timestamp was `2026-10-03T10:14:03+02:00`.
The locked frontend audit reported zero vulnerabilities at every severity.

The native lock SHA-256 is `f7888a62236f6ec5514f496a73cc84d27796b69328dc65cf00333a4cf3e45d96`.
The frontend lock SHA-256 is `7830342279addd8dd5dab7ed82d66aa6574b0e70839b9c326b46225394c1f68e`.
Neither audit replaces the dependency license inventory or bundled frontend notice gate.
The frontend asset collector must preserve the original notices for packages included in emitted chunks.

Evidence: [audit context](evidence/rom-0.0.2/dependencies/context.json), [Cargo results](evidence/rom-0.0.2/dependencies/cargo-audit.json), and [frontend results](evidence/rom-0.0.2/dependencies/npm-audit.json).
