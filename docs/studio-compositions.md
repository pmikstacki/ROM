# Compose a host workspace

These APIs are 0.1.0 candidates. The release and original-consumer acceptance remain open.
Use public package paths. Import `rom-studio/styles` once in the host entry.

## Installed source packages

Remove aliases that route public imports to a vendored Studio entry or its private `$lib` modules.
Use the package exports for controls, authentication and styles.
Keep one Svelte runtime with `resolve: { dedupe: ["svelte"] }` in the host Vite configuration.

For source-package development, use `optimizeDeps: { exclude: ["rom-studio"] }`.
This keeps TypeScript Svelte modules on Vite's normal transform path.
The isolated Astral trial reproduced an optimizer failure without this setting on Vite 8.3.2 and Svelte plugin 7.3.1.
A successful production build alone does not prove that the development server works.
See [installed consumer evidence](research/rom-0.1.0-astral-installed-adoption-2026-10-08.md).

Import `SessionExpiredError`, `SessionDeniedError` and `createBrowserAuth` from `rom-studio/auth` for the legacy browser adapter.
Clear private views after either confirmed authority-loss error.
Treat transient transport failures separately; they do not establish a confirmed denial.
The session lifecycle entry supplies explicit states for new integrations.

## Public controls

Import controls, including `Badge`, from `rom-studio/controls` or `rom-studio`.
Use `BadgeVariant` for a typed badge variant.
The `rom-studio/styles` export supplies its own TypeScript declaration.
The bundler loads the stylesheet through the same runtime CSS path.

## Responsive details

Import `ResponsiveDetails` from `rom-studio/ui/components`.
Use `rom-studio/ui` for headless helpers; that entry does not import Svelte components.

```svelte
<script lang="ts">
  import { ResponsiveDetails } from "rom-studio/ui/components";
  let open = $state(false);
  let opener = $state<HTMLButtonElement | null>(null);
  let summary = $state<HTMLButtonElement | null>(null);
</script>

<button bind:this={opener} onclick={() => { open = true; }}>Inspect</button>
<button bind:this={summary}>Summary</button>
<ResponsiveDetails
  bind:open
  title="Inspection"
  description="Inspect the selected equipment."
  closeLabel="Close inspection"
  {opener}
  fallbackFocus={() => summary?.focus()}
>
  <!-- Supply host controls and explicit labels here. -->
</ResponsiveDetails>
```

Supply `title`, `description`, `closeLabel`, and a content snippet.
The default breakpoint is `(max-width: 1279px)`. The host can supply another media query.
Desktop details use a named nonmodal region. Mobile details use a right-side modal.
The editor stays mounted across breakpoint changes. Draft persistence and mutation recovery remain host responsibilities.
Closing returns focus to a connected, enabled opener. Supply `fallbackFocus` when that opener can disappear.
Two mobile details lifetimes retain the shared body scroll lock until both close.
Host labels update without replacing the editor. Reduced-motion mode shows complete content without transition animation.

## Conversation layout

Import `ConversationLayout` from `rom-studio/ui/components`.
Supply a parent with a bounded height.
Use separate localized labels for the conversation and its scroll region.

```svelte
<script lang="ts">
  import { ConversationLayout } from "rom-studio/ui/components";
  let draft = $state("");
</script>

<div style="height: min(100dvh, 48rem)">
  <ConversationLayout
    label="Conversation"
    bodyLabel="Messages"
    expansion={{ expandLabel: "Expand conversation", collapseLabel: "Collapse conversation" }}
  >
    {#snippet header()}<h2>Conversation</h2>{/snippet}
    {#snippet body()}<p>The host supplies authorized messages here.</p>{/snippet}
    {#snippet footer()}
      <label>Message <textarea bind:value={draft}></textarea></label>
    {/snippet}
  </ConversationLayout>
</div>
```

The component owns layout and keyboard access to its scroll region.
Header, history and footer have separate height limits.
The footer does not shrink. Oversized footer content scrolls within its limit.
Optional `history` content uses the same frame. Desktop expansion does not replace child snippets.
Mobile layout fills the available width and applies safe-area padding to the footer.
If the focused expansion control disappears at the mobile breakpoint, focus moves to the named frame.

The host owns submission, IME handling, authorization and durable recovery.
Bind `bodyRef` to compose the existing follow-latest helper.
Use `expanded` for controlled expansion and the CSS properties `--rom-conversation-width` and `--rom-conversation-expanded-width` for width limits.
This component does not replace `ResponsiveDetails` or its modal focus contract.

Browser checks cover Chromium and WebKit with bounded viewport sizes.
They do not establish physical mobile-keyboard behavior or human usability acceptance.
Installed-package acceptance and the full release verifier remain separate gates.

## Disposable computations and selection

Import `createLatestRequest` and `createSelection` from `rom-studio/ui`.
Use latest requests for disposable previews. Use `rom-studio/recovery` for durable mutations.

Supply a detached value clone, a sanitized error classifier, and a state callback.
Each `run(identity, task)` invalidates the previous result owner. Late results cannot replace current state.
Cancellation is cooperative. The host must bound tasks that ignore their abort signal.

Selection uses exact host IDs. Do not parse an ID into a display title or permission.
Supply a catalog, initial IDs, and `multiple`. Read `selected` after each change.
Use native buttons for selection controls. Keep nested actions outside the selection button.

## Observation ownership

Import `createObservation` from `rom-studio/observe`.
Pass the existing client's `observe()` iterator as the source. Do not add a second stream parser.
Supply an explicit public scope or an exact principal and authority generation.
Missing principal data must not imply public access.

The host supplies detached bounded clones, exact encoded byte measurement, limits, and finite retry ceilings.
Use the ROM codec for exact wire values. Ordinary JSON cloning can lose those values.
Elapsed retry limits govern reconnection admission. They do not limit a fresh observation's lifetime.
The source adapter must bound connection acquisition and honor cancellation.

Change authority generation when rebinding renewed authority. Changed scopes clear rows before new work.
Exactly equal scopes can retain explicitly stale rows after transient failure.
Denial clears rows and invokes `onAuthorityLost`. Neither scope nor cursor grants authorization.

Subscriber failures remove the subscriber and report a sanitized callback code when `onCallbackError` is supplied.
Returned Promise rejections are consumed. Removal happens when rejection settles, not when the Promise is created.
Other notifications can arrive while a callback Promise is pending. There is no asynchronous notification coalescing guarantee.
Keep view-state updates synchronous. If a callback awaits work, fence its own effects against current authority and ownership.
Diagnostic failures cannot interrupt the observation lifecycle. Dispose the controller when its host unmounts.

## Stored layouts

Import `validateLayout` from `rom-studio/ui`.
Pass an explicit catalog with minimum and maximum width and height for each exact widget ID.
Supply positive integer `columns`, `maxRows`, and `maxItems`.

The accepted item shape is `{ id, x, y, width, height, visible }`.
Validation rejects unknown or duplicate IDs, unexpected fields, invalid values, and entries outside grid or catalog bounds.
A valid result contains detached frozen items. Invalid stored data returns a code; invalid host configuration throws `TypeError`.
The validator does not select a fallback, persist data, resolve overlap, or authorize widgets.
The host owns reset choices, layout algorithms, preferences, and principal-bound persistence.

## Verification limits

The installed checkout-source fixture passed type checks, build, and 18 browser cases across Chromium and WebKit.
It tests public controls, details composition, preview ownership, selection, and synthetic observation scope changes.
Its source archive was copied from a checkout. It does not establish clean release provenance or real server authorization.
The later layout candidate passed five behavior tests and its public Node entry check.
Actual HTTP portal flows, original-consumer acceptance, and the final artifact verifier remain required.
