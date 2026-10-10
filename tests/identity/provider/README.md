# Isolated production identity fixture

Status: Task 1 scaffold and isolated provider authoring runs have executed. Public ROM authentication admission remains open.
The [research](../../../docs/research/rom-0.1.0-production-identity-admission.md) and [plan](../../../docs/superpowers/plans/2026-10-07-production-identity-admission.md) define the remaining work.

Run the pure Node tests:

```sh
node --test tests/identity/provider/*.test.mjs
```

`admission.mjs` reserves new evidence directories, compares complete frozen npm locks, checks exact source inventories, and compares provider identities.
Source admission rejects symbolic links, non-regular files, unexpected files, changed files, and traversal.
It limits inventories to 20,000 files and 40,000 filesystem entries. Files are at most 8 MiB; total content is at most 128 MiB.
The expected inventory must come from independently verified extracted source. A caller-supplied hash list does not authenticate an artifact.

`evidence.mjs` checks a closed public result schema and exact scenario coverage across SQLite/redb and Chromium/WebKit.
It rejects partial cases, skipped cases, duplicate cases, source drift, bootstrap installation, failed drains, and unexpected process termination.
An owned intentional SIGTERM shutdown is distinct from an unexpected signal.
The schema checker does not establish that reported processes, provider, TLS, or browsers actually executed.
The future runner must derive those records from actual execution and independently admitted source/tool identities.
Do not use these unit fixtures as release evidence.

Evidence writes use exclusive file creation and finite JSON sizes. A private-field guard rejects common credential fields and raw bearer diagnostics.
This guard does not discover every possible secret encoded under an unrelated field name.
Use the closed final schema and reviewed source/provider identity schemas. Never submit raw upstream bodies or environment dumps.

`supervision.mjs` registers separately owned process groups; it does not launch a child or daemon.
`launch.mjs` starts an approval wrapper before the requested command. It records Linux group, session, start time, and boot identity.
The command starts only after group ownership and supervision admission succeed. A stale identity cannot authorize a signal.
Launch tests use actual Node child processes. Registry tests also use in-memory event emitters for deadline and drain faults.

The registry bounds concurrent children, total retained child records, deadlines, grace periods, drain periods, and aggregate output.
Output is counted and discarded. A child that fails to close produces `drained: false`, which admission rejects.
Records remain available after completion, and a completed child's concurrency slot becomes available.
PID reuse within one registry is rejected rather than silently replacing an earlier record.

The local host has Podman 5.2.3. Its `docker` command reports the same version.
The `rom-dev` environment has no Docker/Podman command. This is read-only command/version evidence, not an engine startup result.
The allocated engine is daemonless host Podman with explicit fresh root/runroot paths, vfs storage, and `--remote=false`.
Clear inherited engine connection variables before a future invocation. Use unique networks and project identities.
The coordinator allocated `/var/tmp/rom-010-authentik-20261007` and loopback ports 44389 through 44393.
Actual isolated information queries passed with zero images and containers. A separate mount/network/PID namespace prerequisite passed.
Image acquisition and provider launch remain separate gates. Repeat the listener check before binding an allocated endpoint.
Do not use a default engine store or the consumer's live deployment.

The acquisition store uses a new fixed-size 12 GiB ext4 backing filesystem. Store, temporary, and download paths stay inside it.
The backing file, mounted filesystem, earlier empty-store evidence, and failed attempts are retained.
The image acquisition monitor stops with 1 GiB available headroom. The filesystem enforces the hard capacity independently of that monitor.
The actual vfs acquisition reached that resource guard before completing. Its partial layers remain retained.
A separately allocated overlay store passed a private namespace read/write probe and both digest-pinned pulls within its own 12 GiB cap.
The updated total retained envelope is 32 GiB: two acquisition stores and 8 GiB for run evidence.
An isolated Authentik/Postgres authoring stack reached readiness and exposed original discovery and JWKS responses.
Image acquisition alone does not establish an OIDC response or browser journey.

`registry.mjs` verifies manifest and configuration byte digests before retaining public image identities.
Manifest and token requests reject redirects. Blob downloads permit one redirect to the exact observed CDN for that registry.
The redirected request has no registry authorization header. Signed redirect URLs and image environment values are not retained.
Image identities and source revisions are separate checks. An empty image revision label does not establish a source binding.

The provider stack uses an owned cgroup hierarchy with aggregate limits of four CPUs, 4 GiB memory, and 512 processes.
A separate browser cgroup enforces its 4 GiB allocation. Container IDs, process birth identities, namespace identity, and limit membership are recorded.
The run filesystem has a separate hard 8 GiB capacity. Failed runs and stopped synthetic volumes remain retained.

Original Authentik JWKS metadata contains an 86-byte key ID and registered certificate fields.
The public SDK initially rejected the original key ID. Fresh current-time verification passed after the shared bound correction.
Early stock-flow, explicit-grant, and synthetic-user provisioning gaps belong to the fixture.
The authoring callback uses borrowed producer Playwright tooling and local HTTP. It cannot satisfy extracted-artifact, trusted-TLS, or two-engine admission.
The standalone SDK fixture compiled and executed against the original token and key material.
Both actual public StudioHost callbacks subsequently returned HTTP 401 before the parser correction.
The original-JWKS parser and all affected Host/auth tests now pass; fresh corrected public callbacks remain pending.
Current-authority, trusted-TLS, two-engine and packaged-consumer cases remain open.

`browser-paths.mjs` checks the UTF-8 Unix socket path budget before launching Chromium.
Long temporary paths produced a retained fixture SIGTRAP. Use a short exclusive temporary directory inside the hard run filesystem.
The Host fixture receives an explicit execution-host asset path. It does not embed the build-container mount path.

`browser-storage.mjs` derives directory realpaths, device identities, inodes and filesystem observations from actual owned paths.
It rejects sibling prefixes, symbolic links, non-directories and another device. The caller still supplies the independently admitted run root.
The Host runner records those observations before browser launch; the browser records its actual persistent profile after launch.
The cgroup drain guard reads `cgroup.events` and rejects live descendants after process drain.
These new guards passed unit tests. Their integrated lifecycle and TLS executions remain separate acceptance work.
