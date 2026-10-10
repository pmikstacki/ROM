# Upgrade identity boundaries

Date: 2026-10-09. Status: source investigation and primary-source review; the integrated upgrade test remains open.

## Observed failure

R11 attempt `7082` stopped when the accepted application's OIDC callback returned HTTP 401 instead of 303.
The browser diagnostic recorded no request failures. It did not identify the exact native rejection step.
Recovery `4423` restored the complete provider configuration. Root compared both saved and restored records byte for byte.
Provider `63443` then stopped normally. These results establish recovery of the test configuration, not application upgrade acceptance.

The accepted source is `584c01b614127b3f62799f1a26a1cdf3f734c3dc`.
Its JWK parser rejects certificate metadata and key identifiers longer than 64 bytes.
The retained Authentik public-key fixture contains certificate metadata and an 86-byte key identifier.
This proves a source compatibility barrier. It does not attribute the actual callback failure to one particular check.

The [accepted release record](rom-0.0.3-release-completion.md) describes an experimental demo provider and in-memory grants.
It does not establish production Authentik interoperability for 0.0.3.
The accepted Host's sessions also reside in memory. Database restoration cannot recreate a browser session or extend credential expiry.

## Primary-source constraints

[OpenID Connect Core, section 5.7](https://openid.net/specs/openid-connect-core-1_0.html#ClaimStability) identifies an end user through the issuer and subject together.
Linking two verified principals to one local User does not make their provider identities equivalent.
ROM receipt replay must therefore retain the original verified principal. A local display name or email cannot replace that identity.

[RFC 7517, sections 4 and 5](https://www.rfc-editor.org/rfc/rfc7517.html#section-4) require ignoring unrecognized JWK and JWK Set members.
The maintained parser accepts specified certificate metadata but still rejects other unknown fields.
This remains a restricted provider profile, not general RFC 7517 interoperability.
Its key acquisition, selected algorithms, and metadata limits must remain explicit in the release support matrix.
Accepting certificate metadata does not establish certificate trust; this profile constructs the RSA key from its modulus and exponent.

## Integrated test design

Keep the unchanged accepted human OIDC fixture for baseline and restored-application authentication.
Keep the unchanged Authentik provider for current production-profile authentication.
Both issue genuine credentials. The relay must preserve their response bodies, key identifiers, signatures, and issuer identities.

The baseline creates Resources, receipts, unfinished Work, and the B0 backup through the accepted public application.
The current application reauthenticates the legacy principal and replays its original requests.
It separately authenticates the Authentik principal for new traffic and unknown-outcome recovery.
An explicit managed IdentityLink can associate each verified principal with the local owner.
A cross-provider replay test must not disclose the legacy receipt as the Authentik principal's result.

After restoring B0, the unchanged accepted application reauthenticates through its supported legacy provider.
It must preserve exact baseline data and receipts, expose post-B0 losses, and avoid repeating previously delivered external effects.
The test must also cover authorized streams, permission revocation, reauthentication, and explicit incompatible-downgrade rejection.
Restored Authentik interoperability is not claimed for the unchanged old Host.

## Remaining proof

First test the combined relay, listener ownership, principal ledger, and failure paths without modifying accepted source.
Then compile source-bound private application facades and verify genuine legacy login before another Authentik window.
The complete integrated test must retain its stage order, deadlines, recovery comparisons, and traffic assertions.
No live provider, build, or API execution is authorized by this document alone.

The [release gates](rom-0.1.0-current-release-gates-2026-10-08.md) distinguish current results from final acceptance.
The implementation plan and source witnesses remain under `.superpowers/rom-010-upgrade-refresh-prep/`.
