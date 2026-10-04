# Independent consumer lock alignment

The integrated native verifier stopped at compile-failure fixtures after workspace tests, documentation and the consumer example passed.
The fixture manifests already used 0.0.2, but their independent lockfiles still recorded the earlier local package versions.
Cargo rejected `--locked` before the intended compiler diagnostic.

Both independent lockfiles now identify local ROM packages and fixture packages as 0.0.2.
External dependency versions and checksums remain unchanged.
The offline refresh initially selected a newer libc version for one fixture. That unrelated update was removed before acceptance.

The complete compile-fixture verifier then passed all intended negative diagnostics and positive renamed, documented, input and versioned cases.
This focused result does not claim that the earlier complete native command passed.
The complete clean-source release producer must run the full native gate again.

- [Integrated failure](evidence/rom-0.0.2/client/native-final-integrated-red.log)
- [Offline lock refresh](evidence/rom-0.0.2/client/compile-lock-refresh.log)
- [Successful compile fixtures](evidence/rom-0.0.2/client/compile-lock-green.log)
