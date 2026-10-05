# ROM 0.0.3 catalog integration review

Reviewed on 2026-10-05 against foundation `6d3b6b2` and the uncommitted release work.
This is a source and focused reproduction review. It is not final release acceptance.

The review covers App authentication, attachment navigation, semantic codec agreement, default renderer composition, and package source routing.
The acceptance contract is [the release plan](../superpowers/plans/2026-10-05-rom-0.0.3-release.md) and [the Studio specification](../../openspec/changes/improve-studio-ergonomics-0-0-3/specs/studio-presentation/spec.md).
The [quality gates](../quality.md) supply the module and shared-contract criteria.

## Contract defects found and resolved

### Stale expiry replies could clear a replacement session

The new expiry branch in `createBrowserAuth.refresh()` cleared session authority before checking its captured epoch.
A delayed expired response could therefore clear the CSRF token of a valid replacement session.

The focused reproduction used this sequence:

1. Accept the first authenticated session.
2. Start another refresh and retain its response.
3. Log out and accept a replacement session.
4. Return the old expired response.

Before the fix, the replacement CSRF token became absent.
The root moved the epoch guard immediately after the awaited session request.
The same reproduction now preserves the replacement token and rejects the old response with `Session changed.`
The named regression is `an old expired response cannot clear replacement session authority` in [auth-expiry.test.ts](../../studio/tests/unit/auth-expiry.test.ts).

### URL whitespace rules differed between Rust and TypeScript

The initial TypeScript URL helper used JavaScript `\s`.
The Rust URL codec used `char::is_whitespace()` and `char::is_control()`.
Those sets differ for U+FEFF.

For `https://example.com/?q=U+FEFF`, the frontend rejected the candidate.
The compiled Rust `Url::new` accepted it and encoded `https://example.com/?q=%EF%BB%BF`.
The focused Rust reproduction linked an existing `rom-fields` artifact; it did not rebuild the workspace.

The root replaced JavaScript `\s` with Unicode `White_Space` while retaining the explicit control-character exclusions.
The same TypeScript reproduction now returns the Rust canonical encoding.
The shared fixture now includes an accepted U+FEFF case.
Both executable codec checks consume that fixture.

## Source review results

| Boundary | Result |
| --- | --- |
| Authentication | The corrected epoch guard runs before expiry handling or session mutation. Login still uses configured provider URLs and validated provider metadata. |
| Reference lookup | The controller queries disclosed kinds through the current SDK session. Results are bounded and stale replies cannot update the picker or main navigation. |
| Attachment capability | The page requires both a returned capability and a disclosed Resource descriptor. Disposal aborts outstanding capability work. |
| Attachment recovery | The panel blocks navigation during preparation, pending work, and unknown outcomes. Retry retains the controller's frozen file, reservation, or detach ID. |
| Attachment disclosure | Details iterate disclosed descriptor fields. Human titles use the shared resolver, and exact IDs remain separate. |
| Semantic wrappers | `ValueEditor` and `ValueDisplay` strip declared codec wrappers before choosing a leaf renderer. Unsupported identities preserve protected generic behavior. |
| Default renderers | The registry imports the finite semantic renderer directly. Custom registration remains explicit and can override a built-in renderer without remote code loading. |
| Source packaging | Studio copy routing includes the complete source tree and locked npm inputs. Rust package discovery includes maintained crates automatically, including `rom-fields`. |

The review found no additional blocker in these source boundaries after the two corrections above.
This conclusion does not cover changes added after this review snapshot.

## Acceptance gaps at this snapshot

The SQLite/redb native stories exercise canonical codecs, actions, authorization, rollback, receipt replay, and reopen recovery.
The browser component stories exercise controls through intercepted fixture transport.
Those results do not establish the full catalog through actual HTTP hosts and production assets.

The maintained demo now declares all ten semantic leaves and typed action inputs.
Its new host journey still needs to exercise create, patch, action, canonical timestamp queries, exact decimal/JSON text, and rejected changes.
The relation picker also needs an actual authorized target query in that journey.
The new `semantic-workflow.ts` task will add this acceptance; it was not executed for this report.

Final Chromium/WebKit acceptance, extracted consumer checks, release version identities, clean-source artifact verification, and preview deployment remain release gates.
The open [OpenSpec tasks](../../openspec/changes/improve-studio-ergonomics-0-0-3/tasks.md) must be reconciled with executed evidence.
No source inspection or isolated browser result closes those gates.

## Focused evidence and limits

The reviewer reproduced both defects before correction and reran each minimal reproduction after the root fix.
The reference picker also passed 11 Chromium development scenarios, the full unit suite, and controller regression tests.
See [reference source facts](evidence/rom-0.0.3/reference-picker/source-facts.json) for commands, source hashes, and limits.

The component build and full native verifier are separate root/adoption-agent acceptance runs.
The initial review did not establish packaged execution, WebKit execution, or preview acceptance.
The follow-up below records later host verification.

## Actual host follow-up

The reviewer added a semantic catalog workflow against the actual human session and production Studio assets.
The matching 0.0.3 binary passed all 28 standard runtime cases across SQLite/redb and Chromium/WebKit.
The explicit custom renderer build passed four further cases across both databases and browsers.
See [runtime source facts](evidence/rom-0.0.3/reference-picker/runtime/source-facts.json) for the immutable binary, asset hashes, commands, and retained failures.

The workflow covers all ten codecs through creation, patching, and typed action inputs.
It checks exact decimal and JSON text, rejected mutation journal stability, canonical timestamp filters, authorized reference choices, and canonical labelled enums.
These results close the actual-host catalog gap for the recorded source and assets.
The screenshot review then found a separate desktop UnitValue layout defect.
After the width correction, fresh assets passed four semantic and four custom renderer cases across both databases and browsers.
Fresh screenshots show the magnitude and unit together in the desktop inspector.
The final [synthetic screenshots](evidence/rom-0.0.3/studio-screens/source-facts.json) show the corrected human-title Resource header and visible magnitude.
Their receipt checks the unchanged source and records exact production assets and immutable binary hashes.
Earlier captures remain in the evidence directory.
The recorded earlier source and asset results remain valid development evidence.
Extracted artifacts, preview deployment, and full release acceptance remain separate gates.
