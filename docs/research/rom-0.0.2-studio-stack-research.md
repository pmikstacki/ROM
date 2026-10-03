# ROM 0.0.2 Studio stack research

Date: 2026-10-03. Status: research and proposed acceptance criteria.

This report applies the repository writing rules and the ASD-STE100 fallback guidance. It does not certify compliance with the standard.

## Evidence boundary

The investigation read current official documentation and npm package manifests. It also read the existing Studio frontend, controls, and client-contract reports.
No frontend packages were installed. No browser, compiler, upstream package, or product acceptance tests were executed for this report.
A GitHub commit API request returned non-JSON content. This report makes no new claim about the latest upstream commit.
Prior maintenance evidence remains in [the controls report](rom-studio-controls-research.md).

The user selects Svelte and shadcn-svelte for this release. Studio must operate on the real Rust Resource pipeline.
A Resource definition remains the authority for fields, actions, codecs, and policies. Studio does not create a second resource schema.

## Recommendation

Use a Svelte 5 and Vite static SPA. Import a small selection of actual shadcn-svelte component sources.
Keep the browser protocol client independent of Svelte. Use one renderer registry for generic views and curated screens.
Serve the production build through the permanent Rust/static service boundary. Keep authentication and mutation decisions in ROM.

This recommendation minimizes runtime services. A Studio data browser does not require public-page SEO or a JavaScript backend.
It does require authenticated discovery, lossless wire codecs, bounded reads, and recovery controls.

## Application shell comparison

| Approach | Useful capability | Cost and release decision |
| --- | --- | --- |
| Svelte + Vite SPA | Compiled components and static assets; direct calls to Rust. | Recommended. Add a small explicit navigation model. Test reloads and the configured base path. |
| SvelteKit static SPA | File-based routing and layouts; static adapter with fallback. | Valid alternative when several route families justify Kit. It adds Kit and adapter compatibility checks. |
| SvelteKit SSR | Server rendering and a JavaScript request lifecycle. | Defer. It adds a server without removing Rust policies, actions, or persistence. |

The official shadcn guide supports Vite installation. [Vite installation](https://shadcn-svelte.com/docs/installation/vite).
SvelteKit supports static output and an SPA fallback. Its documentation describes the performance costs of client-only rendering.
Those public-page costs matter less for this authenticated administration application. [Static adapter](https://svelte.dev/docs/kit/adapter-static), [SPA guide](https://svelte.dev/docs/kit/single-page-apps).

Vite builds static production assets. `vite preview` is a local preview command, not a production server.
The permanent preview must use a supervised service and explicit asset paths. [Vite deployment](https://vite.dev/guide/static-deploy).

## Verified package declarations

These values came from read-only requests to the npm registry on the report date. They are compatibility declarations, not installed combinations.

| Package | Observed version | Relevant declaration |
| --- | --- | --- |
| [svelte](https://registry.npmjs.org/svelte/latest) | 5.57.1 | Node >=18; MIT. |
| [vite](https://registry.npmjs.org/vite/latest) | 8.3.2 | Node ^20.19.0 or >=22.12.0; MIT. |
| [@sveltejs/vite-plugin-svelte](https://registry.npmjs.org/@sveltejs/vite-plugin-svelte/latest) | 7.3.1 | Svelte ^5.46.4; Vite 8; Node ^20.19, ^22.12, or >=24; MIT. |
| [shadcn-svelte](https://registry.npmjs.org/shadcn-svelte/latest) | 1.7.0 | Svelte ^5.0.0; MIT. This is the CLI version. |
| [bits-ui](https://registry.npmjs.org/bits-ui/latest) | 2.19.5 | Svelte ^5.33.0; Node >=20; @internationalized/date ^3.8.1 peer; MIT. |
| [tailwindcss](https://registry.npmjs.org/tailwindcss/latest) | 4.3.3 | MIT. |
| [@tailwindcss/vite](https://registry.npmjs.org/@tailwindcss/vite/latest) | 4.3.3 | Vite 5 through 8 peer; MIT. |
| [@sveltejs/kit](https://registry.npmjs.org/@sveltejs/kit/latest) | 3.0.0 | Node >=22.17; Svelte ^5.57.1; Vite ^8.0.12; TypeScript ^6.0.0; MIT. |
| [@sveltejs/adapter-static](https://registry.npmjs.org/@sveltejs/adapter-static/latest) | 4.0.0 | Kit ^3.0.0-next.0 peer; MIT. Installation must resolve this combination. |
| [@tanstack/svelte-table](https://registry.npmjs.org/@tanstack/svelte-table/latest) | 9.2.4 | Svelte ^5.0.0; Node >=20; MIT. |
| [sveltekit-superforms](https://registry.npmjs.org/sveltekit-superforms/latest) | 2.31.0 | Svelte 5 included; Kit peer 1.x or 2.x excludes Kit 3; MIT. |
| [formsnap](https://registry.npmjs.org/formsnap/latest) | 2.0.1 | Svelte ^5.0.0; Superforms ^2.19.0; MIT. |
| [@playwright/test](https://registry.npmjs.org/@playwright/test/latest) | 1.63.0 | Node >=20; Apache-2.0. |

The research shell reported Node 22.16.0. This satisfies the listed Vite engine but does not satisfy Kit 3.
A Kit choice must update the executable toolchain or select a separately verified supported version combination.
Do not force-install Superforms with Kit 3. Use ordinary Svelte state and the ROM operation encoder for this release.

Pin the resolved versions in the lockfile. Record registry component source hashes separately from the CLI version.
The build gate must execute the resolver, type checks, production build, browser tests, advisory review, and license inventory.
The manifest review alone establishes none of these results.

## shadcn ownership and maintenance

shadcn-svelte distributes editable component source over Bits UI and Tailwind. It is a community Svelte port of shadcn/ui.
The copied component layer belongs to ROM after import. Dependency updates can provide primitive fixes but do not update copied source automatically.
[Project introduction](https://shadcn-svelte.com/docs).

Import only components the release uses: Button, Input, Label, Field, Table, Alert, and selected navigation or dialog controls.
Record the source URL, revision or downloaded hash, license, and local changes for each imported component family.
Keep the imported presentation components separate from Resource rendering and wire codecs.

Do not call a handwritten look-alike a shadcn import. Do not regenerate modified sources with blanket overwrite commands.
Review upstream differences before an update. Run keyboard and interaction tests after changes to local wrappers or Bits UI.
The existing controls report retains the previous upstream activity evidence. New package observations establish availability, not a guarantee of future maintenance.

## Generic and reusable composition

Use three layers with explicit ownership:

| Layer | Responsibility |
| --- | --- |
| Browser client | Decode discovery and responses; encode actions; own session generations, command identity, cancellation, and stream recovery. |
| Resource UI | Render descriptor fields; preserve drafts; build permitted filters; present structured errors, conflicts, and outcomes. |
| shadcn primitives | Provide controls, layout, focus behavior, and consistent presentation. |

A registry maps a semantic field type and display mode to an approved Svelte component.
Supported modes include editor, detail, and table cell. The generic host owns labels, errors, permission state, and operation presence.
Unknown types produce an explicit read-only fallback or unavailable editor. Metadata never loads arbitrary executable JavaScript.

Svelte 5 supports component-valued variables in runes mode. TypeScript can express a shared `Component<Props>` contract.
Snippets provide deliberate layout overrides without copying the field host. [Svelte migration](https://svelte.dev/docs/svelte/v5-migration-guide), [Component typing](https://svelte.dev/docs/svelte/typescript).

Use the same ResourceTable, ResourceForm, ActionForm, and FieldHost in the resource browser and curated user/settings pages.
A second or third ordinary Resource must appear without a dedicated route or form.
Plugin-defined Resource origins must not select a different mutation path.

Start with a semantic table over shadcn Table primitives. Add TanStack only if selection, column controls, or other behavior warrants the dependency.
Sorting, filtering, and pagination must express ROM queries. A client must not sort one downloaded page and present it as a global ordering.

## Codec and draft semantics

Keep unchanged, explicit null, set value, and explicit remove distinct when the server contract supports them.
False, zero, and the empty string remain values. A disabled control does not independently define the operation encoder.
Keep the authoritative snapshot, revision, and draft separate. Live refresh must not overwrite edits silently.

Use exact textual representations for large integers and decimals when the wire codec requires them.
Do not route every numeric field through JavaScript `Number`. Reject unsupported codec versions before an action.
The renderer registry is not a codec registry: presentation cannot change command meaning.

The client must bind a response to the requested kind, ID, command, revision, and stream generation where applicable.
Keep an uncertain mutation's idempotency key for outcome lookup or same-key retry.
Do not replay it under another principal. A timeout is not confirmation that the mutation failed.

These requirements extend the retained [Studio client contract](rom-studio-client-contract.md) and the release's Resource guarantees.
They are recommendations for executable tests, not browser results from this report.

## Authentication and live updates

Use the real ROM authentication boundary. Do not ship a browser with an embedded administrator credential.
A local development credential entry can be explicit and memory-only. Label its scope and clear it on logout.
Provider redirect support requires its own verified browser flow; a client-credentials service token is not that flow.

For native EventSource, the constructor supports URL and credentials options, not arbitrary Authorization headers.
Bearer authentication therefore needs a verified fetch stream or a separate secure session design.
Do not move durable bearer credentials into URL parameters. [EventSource interface](https://html.spec.whatwg.org/multipage/server-sent-events.html#the-eventsource-interface).

Policy loss must clear restricted rows, descriptors, fields, and drafts. Close old subscriptions on session changes.
Tag pending responses with session and query generations. Reconnect must obtain an authorized baseline when continuity is uncertain.
A live query and a journal cursor have different lifecycle contracts. Keep those distinctions in the reusable client.

## Accessibility and browser acceptance

shadcn Field composes labels, controls, descriptions, and errors. Correct relationships still depend on the assembled application.
[Field documentation](https://shadcn-svelte.com/docs/components/field).
Test label association, keyboard navigation, focus after errors, dialog focus restoration, and visible pending/outcome states.
Do not use color as the only state indicator.

Tailwind 4 lists Chrome 111, Safari 16.4, and Firefox 128 for its core browser features.
Additional CSS features can require newer engines. [Browser compatibility](https://tailwindcss.com/docs/compatibility).

Run Playwright against the production build and real backend. Use Chromium and WebKit at minimum for the Mac preview path.
Playwright's WebKit build is not the user's actual Safari executable. Record that limit.
Include automated accessibility scans and deliberate keyboard scenarios. Automated scans cannot establish full accessibility.
[Browser engines](https://playwright.dev/docs/browsers), [Accessibility tests](https://playwright.dev/docs/accessibility-testing).

## Required executable trials

| Trial | Acceptance evidence |
| --- | --- |
| Locked stack | Clean install, Svelte/TypeScript checks, build, dependency audit, and license inventory succeed on the declared toolchain. |
| Generic rendering | Two distinct Resources render from authorized discovery; a third appears without kind-specific view code. |
| Exact operations | Omitted/null/false/zero/empty and exact custom codecs preserve request meaning through the real Rust backend. |
| Shared mutation guarantees | Generic forms cannot bypass transition validation, protected deletion, or field policy. |
| Conflict and unknown outcome | Draft survives a stale revision; a lost response recovers the same receipt without another mutation. |
| Session boundaries | Discovery and cached data do not cross principals; revocation clears affected views and rejects new operations. |
| Live lifecycle | Filter membership, disconnect, reset, generation change, and reconnect produce authorized current results. |
| Human author workflow | An external application adds a Resource and custom renderer through public contracts; no repository or endpoint is copied. |
| Packaged preview | Extracted release serves HTML, hashed assets, and API from the configured base path; direct navigation and restart work. |
| Accessibility | Keyboard scenarios and scans run on actual forms, tables, errors, and dialogs; findings and limits are recorded. |

For each trial, record source commit, lockfile hash, toolchain, command, backend, browser engine, and exit status.
Keep documentation inspection, unit trials, real backend acceptance, and human review as separate evidence categories.
