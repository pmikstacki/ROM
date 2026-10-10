/** Descriptor-driven ROM forms using the shared ROM UI controls. */
export { default as ResourceForm } from "./lib/resources/ResourceForm.svelte";
export { default as ActionForm } from "./lib/resources/ActionForm.svelte";
export { default as FieldHost } from "./lib/renderers/FieldHost.svelte";
export type { ResourceFormSubmission } from "./lib/resources/form-types.ts";
export type { EditorDraft } from "./lib/renderers/editor-draft.ts";
export type { EditorSnapshot } from "./lib/application/editor-drafts.ts";
export { registerRenderer } from "./lib/renderers/registry.ts";
export type { RendererRegistrationOptions } from "./lib/renderers/registry.ts";
export type { FieldRenderer, RendererProps } from "./lib/renderers/types.ts";
