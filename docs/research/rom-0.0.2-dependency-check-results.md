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

## Current Studio frontend lock

The complete shadcn-svelte registry installation and SVAR filter fork changed the frontend dependency graph after the earlier audit.
The current `studio/package-lock.json` SHA-256 is `44a7ef3624f657b82b69731c13c48d69a3054f9c9a18d87a97406d6d9742c7e8` at source `0c40007995ada63f8e3304593ccbacc9deaca4c8`.
On 2026-10-04, npm 10.9.2 ran `npm audit --json --audit-level=low` against that lock and reported zero findings at every severity.
The raw result is `/root/ipi/research/disk-coordination-2026-10-04/ROM-current-frontend-npm-audit.json` (SHA-256 `903aa2a076850ec6325bce8b14d25415d3361068b49a00c6287026ef3728eb23`).
The external `ROM-shadcn-full-registry-install.md` report records the earlier cookie advisory, the pinned `cookie` correction, and the source inventory.
These checks are time-specific. The final release producer must still bind emitted notices to its own asset and package outputs.

## Local maintenance dependency edge

The real-provider maintenance test adds only a local rom-backup dev dependency. No external package version changed. The native lock is now `f288709d12f8c7adc39a5d72cb7253a9d86afa990dd94226724e39262500d992`. The all-features inventory still contains 282 external packages. A fresh Cargo advisory run passed with no matching advisories, using the database recorded in [the new context](evidence/rom-0.0.2/provider-maintenance/dependency-context.json). The [raw result](evidence/rom-0.0.2/provider-maintenance/cargo-audit.json) and regenerated inventory remain separate from the earlier lock record. The frontend lock did not change.
