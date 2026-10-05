# Provider deadline regression

The complete producer failed the provider-auth deadline test after the original verifier drained.
The test uses a 30-millisecond response deadline for both the blocked first call and the subsequent call.
Its final assertion required the subsequent verification to finish before that short deadline.

Instrumentation captured `Err(Overloaded)` in one of 40 repetitions. The other 39 repetitions passed.
See [the original timeout](evidence/rom-0.0.3/auth-deadline/original-timeout.log).
The resolver maps a response timeout to this error while retaining ownership of the admitted job.
This error alone does not prove that capacity remained occupied.

## Corrected proof

The regression keeps the same deadline, authentication instance, and one-slot job tracker.
While the first verifier is blocked, another call remains overloaded and the provider contact count stays at one.
After release, the original job must drain within one second.
The next call must produce a second provider contact and drain through the same tracker within bounded waits.
Only the expected actor or a caller deadline error is accepted. `Denied` and other errors fail.

The second contact proves admission independently of response latency.
A temporary negative control required that contact before releasing the original verifier.
It failed within the bounded wait. See [the negative control](evidence/rom-0.0.3/auth-deadline/capacity-held-red.log).
The original successful verification scenario remains covered under its ordinary deadline.
Production authentication code did not change.

## Verification limits

The restored test passed 40 repetitions. The provider-auth test binary passed 14 tests and reported two ignored subprocess fixtures.
Formatting and diff checks passed.
The [full local verifier](evidence/rom-0.0.3/auth-deadline/full-check.log) passed on the corrected source.
The [extracted-package gate](evidence/rom-0.0.3/auth-deadline/extracted-packages.log) passed using 15 archives.
The complete clean-source producer remains a separate requirement.
The investigation does not establish a production latency bound or complete release acceptance.

Original investigation records remain in `/var/tmp/rom-003-auth-deadline-investigation/`.
The tested file digest is `82cec5815f5888a42ca2c6cfcb769a0dcc5f9b72488d55c4bfac2bef315f1ee9`.
