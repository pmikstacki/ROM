# ROM 0.0.3 package fixture portability

Status: correction passed extracted-package checks; complete release acceptance remains pending.
Date: 2026-10-05. Failed producer source: `83f20c2f44c3bc28474b20b7d5863aadd595aca3`.

The complete producer passed gates one through five. Gate six rejected the copied external consumer.
Its enum discovery test included a fixture from a sibling workspace crate. That path did not exist after extraction.
A separate test of the extracted core found the same ownership error in its negative enum fixture.
These were package defects. They were not failures of the enum wire contract.

The positive discovery fixture now belongs to the consumer's own test directory.
The negative validation fixture now belongs to the core's own test directory.
Studio reads those same files. Fixture content remains unchanged; no duplicate corpus was introduced.
The package gate also executes the extracted core's library tests.

The [producer failure](evidence/rom-0.0.3/package-portability/producer-gate-6-red.stderr.log)
and [extracted core failure](evidence/rom-0.0.3/package-portability/extracted-core-red.log) remain available.
The incomplete producer stage remains at `/root/ROM/dist/.rom-release-stage-CMrj3m`.
Earlier environment failures also remain preserved. Missing linker and OpenSSL tools preceded the isolated target's accepted native gates.

The [corrected package check](evidence/rom-0.0.3/package-portability/extracted-packages-green.log) passed using fifteen extracted library archives.
It includes 95 extracted core tests, the independent consumer, and the reference application.
The full local verifier also [passed after the fixture move](evidence/rom-0.0.3/package-portability/full-check-after-fixture-move.log).

The corrected source still requires the complete producer, independent artifact verification, and persistent preview acceptance.
A source correction or successful partial gate does not complete the release.
