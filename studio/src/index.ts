/** Public source entry for applications that compose ROM Studio. */
export { default as App } from "./App.svelte";
export { createClient, RemoteError } from "./lib/client/client.ts";
export type * from "./lib/client/types.ts";
export { registerRenderer } from "./lib/renderers/registry.ts";
export type { FieldRenderer, RendererProps } from "./lib/renderers/types.ts";
export * from "./controls.ts";

export type { RendererRegistrationOptions } from "./lib/renderers/registry.ts";

export { parseWire, stringifyWire } from "./lib/client/codec.ts";
export * from "./recovery.ts";
export type { StudioAuthProfile } from "./lib/application/session-types.ts";
export { createStudioBootstrap, parseStudioBootstrap } from "./lib/application/bootstrap.ts";
export type { StudioBootstrapConfig, StudioBootstrapStore } from "./lib/application/bootstrap.ts";
