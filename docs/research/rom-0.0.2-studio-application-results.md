# Studio application composition trial

Date: 2026-10-03. Scope: Task 5 browser composition before actual-host acceptance.

## Implemented boundary

The application receives a `RomClient`. Its default client uses the Vite base path plus `api`. The browser does not construct an Actor, retain an access token, or choose a persistence driver.

Authorized discovery supplies the navigation and the Resource pages. The same table, query editor, details, create form, patch form, action form, and deletion control work for each discovered kind. A Resource ID identifies each detail form. Live revisions do not reset its draft. Changed descriptor versions disable its submission through the shared form gate.

The controller owns navigation generations, row-selection generations, request cancellation, mutation outcomes, and live subscriptions. Its state is an immutable snapshot. Svelte uses raw snapshots so it does not proxy the prepared mutation object required by the client's private receipt map.

An unknown mutation blocks another mutation. The outcome panel retries the same prepared object. Query refresh failure does not convert a confirmed mutation into an unknown outcome. Navigation and session guards reject late responses.

The Work page calls the existing authorized operator routes. It exposes inspection, retry, and reconciliation only when the server declares the corresponding capability. Recovery requires confirmation and the inspected work version. An unconfirmed recovery retains its exact request and key. This page does not add another domain entity.

## Browser identity boundary

The browser reads the configured providers and the host session. An authenticated session supplies the CSRF value for the generic client. Cookies remain host-managed. Provider URLs encode the provider ID. A selected primary provider redirects once per tab; the provider buttons remain available if browser session storage is unavailable.

The application checks the session at startup and every 15 seconds. Each session request has a 10-second timeout and a 64 KiB body bound. Changed or denied sessions clear projections, descriptors, pending mutations, and work data. Logout clears local data before waiting for the server. A delayed session response cannot restore its CSRF value after logout.

Live identity expiry allows one revalidation attempt before a new authorized snapshot. Each accepted snapshot resets that consecutive failure budget. Regular finite lease renewals can continue until the original finite session expires. Repeated expiry without a new snapshot stops the stream and clears its projections. This mechanism does not invent a journal cursor or claim indefinite revocation while offline.

## Executed checks

The retained evidence is in [frontend-application](evidence/rom-0.0.2/frontend-application).

- Svelte and TypeScript diagnostics: zero errors and warnings.
- Production build: passed. Component test fixtures are excluded from its entry points.
- Controller and browser-auth Node tests: 12 passed.
- Chromium browser tests: 10 passed. Eight cover shared controls; two cover application composition and unavailable sessions.

The controller tests cover discovery, competing navigation, exact unknown retry, disconnection during discovery, finite lease recovery, repeated lease failure, and three separated lease expiries. The auth tests cover CSRF admission, expired or unauthenticated authority rejection, logout headers, provider admission, and logout during a pending session response.

The application browser test uses the real SDK with explicit intercepted test transport responses. It discovers two different kinds, opens a detail view, changes kind, and signs out. The unavailable-session test confirms that the page displays an error without simulated successful data. These are executable frontend tests. They are not real Rust host or human OIDC acceptance.

## Remaining acceptance

Run the application against the optional Rust host and the human OIDC fixture. Cover a third Resource, a registered custom field, actual commits, live membership, finite lease renewal, revocation, unknown receipts, work control, restart, and another-tab logout. Run keyboard and accessibility checks on the complete connected page. Repair the pinned WebKit runtime described in the earlier compatibility report, then run the required browser acceptance there. No WebKit or production readiness claim is made by this trial.
