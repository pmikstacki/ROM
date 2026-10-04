# Studio shadcn presentation

The owner requires the complete shadcn-svelte component set and consistent generic controls. Existing release authorization permits implementation without further approval.

## Design

Use the official registry with the pinned CLI. Record the registry source and dependency changes. Preserve third-party licenses.

The application shell uses Sidebar.Provider, Sidebar.Root and Sidebar.Inset. A compact header contains the mobile trigger and current page. Resources remain descriptor-driven. Work and Attachments use the same navigation shell.

Shared field editors use shadcn controls. Controlled adapters preserve omit, value, null and remove. Custom codecs retain their renderer registration and lossless integer handling.

The Resource page separates query controls, results and details. Details retain draft revision checks, current authorization and explicit delete confirmation. Presentation changes must not reset an unknown mutation or generate another idempotency key.

## Ownership and tasks

1. Registry: operator_work_protocol owns components/ui, hooks, package manifests and components.json. Install all components with the pinned CLI. Check dependencies, licenses and source provenance. Keep installation scratch within 2 GiB of RAM and retained growth within 512 MiB.
2. Generic views: release_lessons_audit owns renderers, resources, application/ResourcePage.svelte, ResourceDetails.svelte, WorkPage.svelte, AttachmentPage.svelte, attachments/AttachmentPanel.svelte and their presentation tests. Use official controls after registry installation. Preserve existing controller and client behavior.
3. Shell: root owns App.svelte, new presentation shell components, app.css and shell tests. Replace stacked mobile navigation with the official responsive Sidebar. Use neutral slate tokens, consistent spacing and visible focus states.
4. Verification: run svelte-check, unit and component suites. Test desktop and mobile layouts, keyboard navigation, focus, current authorization, stale drafts and unknown outcomes. Run both browser engines with the verified confined font and software EGL configuration.
5. Review: inspect screenshots from the actual native host. Check all generic controls and copied component dependencies. Re-run all release gates before publication. Installation or a screenshot alone does not establish release readiness.

## Admission

Retain the 92 GiB shared free-space floor. No Rust or Nix producer is admitted by this plan. Notify coordination before another full producer phase. Preserve failed images and all evidence.

## Acceptance

- The complete official component set is installed; unused components do not become artificial application features.
- Navigation collapses on mobile without pushing the Resource table below a full navigation list.
- Forms expose controlled values, associated labels, validation errors and keyboard interaction.
- A second Resource uses the same controls without a resource-specific controller.
- Shared Resource, action, query, persistence and live semantics remain unchanged.
- Dependency notices include all runtime dependencies added by the registry.
- Both browser engines pass the affected tests and the complete release verifier.
