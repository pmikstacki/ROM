# Disposable ROM Studio visual mock

Question: how should a generic Resource browser coexist with curated account/provider screens and primary-provider login?

Three visual directions share in-memory sample data and behavior: Explorer (sidebar and table), Workspace (top navigation and resource cards), Inspector (compact navigation and side-by-side details). The conversation renderer supplies the variant carousel. This fragment is plain browser markup for rapid design discussion, **not a Svelte/shadcn implementation or proof of their compatibility**. No backend, credentials, external authentication or real configuration writes are involved.

Try Task/Asset resource switching, definition inspection, record editing, User suspension, and IdentityProvider primary/direct-sign-in settings. The sign-in preview switches between the Studio chooser and a simulated provider handoff. A primary provider and automatic handoff are represented separately in this draft so a default choice can still appear on Studio's login screen. This is proposed UI vocabulary, not a finalized auth contract. An external provider return is simulated with a button; the mock never navigates to a provider or collects credentials.

All state is local to each design variant and discarded on reload. Config-source labels are illustrative, not an accepted precedence policy. The data is synthetic. No design has been selected. Do not promote this code into production; use the eventual decision to implement tested Svelte components against the ROM client contract.

Browser smoke verification used headless Chromium: Resource switching, a User action, primary-provider direct-login preview, alternate-provider choice and return to Studio all worked. All three layouts were checked for page overflow at 320, 560 and 1040 pixels; no page-script errors occurred. A desktop screenshot was inspected. This is visual interaction evidence, not backend/authentication or Svelte-library conformance.
