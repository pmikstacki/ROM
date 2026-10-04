/** Public source entry for applications that compose ROM Studio. */
export { default as App } from "./App.svelte";
export { createClient, RemoteError } from "./lib/client/client.ts";
export type * from "./lib/client/types.ts";
export { registerRenderer } from "./lib/renderers/registry.ts";
export type { FieldRenderer, RendererProps } from "./lib/renderers/types.ts";
export { Input } from "./lib/components/ui/input/index.js";
