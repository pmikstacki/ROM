# Shared opened-file admission results

Date: 2026-10-04.

The provider profile and trusted Studio profile now use one named file-admission module: `demo/src/host_files.rs`. The existing provider secret API path stays unchanged.

On Linux, the shared reader opens a file with `O_NOFOLLOW | O_NONBLOCK`. It validates the opened descriptor before reading. It rejects non-regular files, oversized content and invalid UTF-8. Reads are bounded to the configured limit plus one byte. Private reads reject group and other permission bits. Studio credentials additionally require an absolute path and ownership by the process owner. Studio JSON profiles require an absolute path and a regular file. Errors do not contain file contents.

The Studio feature enables the existing pinned optional `libc` dependency. It does not introduce a new dependency version. Non-Linux platforms have no weaker fallback: these reference profiles reject admission.

These files must be immutable within host-trusted parent directories. `O_NOFOLLOW` protects the final pathname component. This reader does not establish trust in every intermediate directory.

## Why the correction was needed

The previous Studio reader checked the pathname before a blocking `File::open`. A replacement between that check and the open could change the target. A FIFO replacement could block before opened-file metadata checks ran. This was a source-backed review concern. No actual pathname-race experiment was performed, and no race failure is claimed.

The existing provider reader already used opened-handle admission with both flags. The correction reuses that mechanism instead of keeping a second weaker reader.

## Executed checks

The complete `rom-demo` tests with `studio,provider-profile` features passed. This includes existing provider behavior and the new shared-reader cases. The new cases admit a private regular file and reject a direct symlink, public permissions, oversized bytes and invalid UTF-8. A stable FIFO with no writer is rejected without waiting for a writer. That FIFO test checks nonblocking admission; it does not simulate the pathname race.

All-target Clippy passed with warnings denied. The Studio-enabled binary built successfully. An actual subprocess launcher probe passed on SQLite and redb with the trusted HTTPS profile. It checked readiness, provider listing and SIGTERM exit code 0. It does not prove a deployed TLS proxy or human browser login through that proxy.

The first test run expected lowercase `denied`; the actual redacted error was `Denied`. That test expectation was corrected. The first all-target Clippy run also found a test-style `err().expect()` use; it was replaced with `expect_err()`. These setup and test-style failures are retained separately from successful behavioral results.

See [the native tests](evidence/rom-0.0.2/secure-host-files/native-results.log), [Clippy and build results](evidence/rom-0.0.2/secure-host-files/clippy-build-results.log), and [the launcher results](evidence/rom-0.0.2/secure-host-files/https-launcher-results.log). The launcher used an immutable copy of the freshly built executable. [Both binary hashes](evidence/rom-0.0.2/secure-host-files/native-binary.sha256) are equal. No new Cargo target was created.
