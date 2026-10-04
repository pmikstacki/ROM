# Studio binary transport

The optional host exposes blob operations beside its generic Resource API.
The client first admits authenticated capabilities for the current session generation.
The response identifies the Resource kind, disclosed store names, limits, and supported operations.
The browser does not choose a hard-coded Resource kind or manufacture an Actor.

The client freezes reservation input before it waits for the transport.
It sends exact integer byte counts and retains the supplied idempotency key.
Upload uses an immutable `Blob` slice with explicit binary content type.
Mutating requests include the session CSRF token and same-origin credentials.
The client does not automatically retry a mutation with an unknown outcome.

Successful mutation responses must identify the admitted Resource kind and submitted ID.
The status must match the submitted operation, and the Resource projection must be present.
Download returns bounded bytes. It does not decode binary content as JSON or accept another content type.
All requests use the existing finite deadline and session cancellation mechanism.

## Independent review corrections

The reviewer reproduced three transport defects and one overlapping-refresh defect.

| Defect | Correction |
| --- | --- |
| Dot IDs normalized into another URL path | Download uses the exact ID in a query parameter. |
| Header rejection left an unused response body open | Rejection requests body cancellation without waiting indefinitely. |
| A response from an old generation populated the new capability cache | Admission retains the original generation and rechecks it before storage. |
| An older successful refresh reopened admission after a newer denial | A request epoch rejects superseded capability responses. |

These defects affected client admission and transport behavior. They did not bypass the host's current Resource authorization.
Returned capability objects cannot widen the client's private limits or approved store list.
A failed refresh closes local admission. Session invalidation makes previous capability admission unusable.

## Executed tests

The final coordinator SDK run passed 63 tests.
Tests cover frozen reservation input, exact integers, binary limits, CSRF, response correspondence, finite completion, pre-abort, and capability invalidation.
The generation regression tests 40 asynchronous completion positions.
The overlapping-refresh test releases an older response after the newer request receives a denial.

Evidence:

- [Missing client implementation](evidence/rom-0.0.2/client/blob-sdk-red.log)
- [URL correspondence regression](evidence/rom-0.0.2/client/blob-headers-route-red.log)
- [Unused response body regression](evidence/rom-0.0.2/client/blob-headers-red.log)
- [Generation regression](evidence/rom-0.0.2/client/blob-generation-red.log)
- [Overlapping refresh regression](evidence/rom-0.0.2/client/blob-capability-refresh-red.log)
- [Final SDK acceptance](evidence/rom-0.0.2/client/blob-sdk-final-green.log)

These unit tests use an injected transport. Actual provider, native host, attachment UI, and process shutdown have separate acceptance gates.
