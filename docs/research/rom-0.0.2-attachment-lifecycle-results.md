# ROM 0.0.2 attachment and process lifecycle results

Date: 2026-10-04.

## Implemented scope

Studio has a reusable attachment page. The page obtains authenticated transport capabilities and finds the resource descriptor by the advertised `resource_kind`. It does not select behavior by a resource kind name. The existing generic table and field renderers display authorized blob Resources.

The attachment controller checks file limits before it reads file bytes. It freezes the bytes, digest, Resource ID, store and idempotency key before the first reserve call. An uncertain reserve result retains the same reservation. An uncertain upload result retains the same publication. Retry requires a user action. The controller does not retry automatically. Disposal clears pending content and denies a late download result.

Store names come from the host's explicit discovery policy. Capabilities describe available operations. They do not grant permission to execute those operations. The host still checks current identity and Resource authorization.

The demo uses the real trusted-folder blob adapter. An opt-in publication barrier uses the existing private Unix socket. The socket directory has mode `0700`; the socket has mode `0600`. No browser or HTTP route exposes the barrier. Private controls remain available while shutdown drains accepted work.

## Executed checks

The new real-host browser suite passed 12 cases: three scenarios on SQLite and redb, in Chromium and WebKit. The suite uses the real Rust host, production assets and the human OIDC provider fixture. See [the raw browser output](evidence/rom-0.0.2/attachments/browser-results.log).

1. Upload: reserve and publish `folder/name`. The test lets the host complete the upload, then discards the response. Explicit retry returns the committed Resource at revision 2. Download returns the exact original bytes. Confirmed detachment changes the Resource state.
2. Process shutdown: hold accepted publication and an independent accepted OIDC token exchange. Cancel both browser waiters. Send the actual native process `SIGTERM`. Release publication while token exchange remains held. The process stays alive. Release token exchange; the process exits with code 0. Restart the same database and sign in again. The attachment is ready at revision 2.
3. Cost sample: render the three seeded Tasks, open their live query, and patch one title. Count API requests and record elapsed time to the next visible frame.

An earlier download test used a native binary built before the canonical download route was added. The UI rejected the returned HTML as an invalid binary content type. Rebuilding the current native source made all four attachment cases pass. The rejection was correct; the test inputs were not aligned.

## Finite cost sample

Each run used three Tasks, one inventory Resource and one held-out custom-field Resource. The Task table contained 15 cells. Each run issued one discovery request, three query requests, one live request, one read request and one invocation.

| Browser | Store | Query to frame (ms) | Stream open to frame (ms) | Mutation to frame (ms) |
| --- | --- | ---: | ---: | ---: |
| Chromium | SQLite | 117.3 | 64.2 | 77.7 |
| Chromium | redb | 118.3 | 64.2 | 79.8 |
| WebKit | SQLite | 83 | 46 | 73 |
| WebKit | redb | 84 | 45 | 75 |

These are one-sample local measurements. They include browser-driver scheduling, network requests, codecs and frame scheduling. They do not isolate renderer CPU cost, measure large tables, compare database query plans, or establish a universal performance result. No latency threshold determines whether these tests pass. Per-run databases, logs and `cost-measurement.json` files remain in `/var/tmp/rom-studio-host-browser-*`.

## Limits

The provider fixture is a loopback development fixture with an in-memory provider store. It is not a production identity service. The attachment test uses local folder storage, not S3. A successful local process test does not prove cross-machine fault tolerance. Full release gates and independent review remain separate checks.

## Independent review correction

The reviewer found that HTTP 500 was treated as a confirmed failure. That discarded the pending file and reservation key. A new controller test reproduced the error. The controller now retains pending work for uncertain HTTP 5xx responses. Explicit `not_committed` and known non-5xx refusals still clear pending work. Six controller tests pass. The download UI also captures the selected Resource ID before it awaits bytes, so a later row selection cannot change the downloaded filename.

After these corrections, all four actual attachment browser cases passed again. See [the review regression](evidence/rom-0.0.2/attachments/http500-red.log), [the controller results](evidence/rom-0.0.2/attachments/controller-results.log), and [the browser rerun](evidence/rom-0.0.2/attachments/review-fix-browser-results.log).

The reviewer also found that the query editor omitted `codec_wrappers`. This could pass a whole list to a registered leaf renderer. The query editor now forwards the wrapper path. A browser regression builds a list filter through its registered leaf renderer. It failed on both engines before the fix and passed on both engines afterward. See [the failure](evidence/rom-0.0.2/attachments/query-wrapper-red.log) and [the passing result](evidence/rom-0.0.2/attachments/query-wrapper-green.log).

## Exact-byte recovery check

The SIGTERM scenario now downloads the attachment after the native process restarts. It compares the recovered bytes with `survives SIGTERM`. The check passed on both stores and both browser engines.

A controlled negative probe changes the first byte of the real post-restart download response. All four cases failed at the byte comparison: expected `survives SIGTERM`, received `rurvives SIGTERM`. The probe does not replace the host, database, upload or restart. It changes only the response bytes after the real host returns them. Without that corruption, all four cases passed. See [the failure](evidence/rom-0.0.2/recovery-bytes/corrupted-download-red.log) and [the passing result](evidence/rom-0.0.2/recovery-bytes/download-green.log).

A separate rerun could not start Studio because a concurrent workspace gate had replaced the shared demo executable with a build without the `studio` feature. That setup failure is retained separately. The passing focused probe used an immutable copy of the freshly built feature-enabled executable. [Both executable hashes](evidence/rom-0.0.2/recovery-bytes/native-binary.sha256) are equal. The production release gate still builds its own current-source executable; it does not use this focused-test override.
