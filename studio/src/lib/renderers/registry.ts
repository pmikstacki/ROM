import type { CodecIdentity } from "../client/types.ts";
import type { FieldRenderer } from "./types.ts";
import { SvelteMap } from "svelte/reactivity";
import SemanticField from "./SemanticField.svelte";
import { semanticKind } from "./semantic-fields.ts";
export interface RendererRegistrationOptions {
  layout?: "inline" | "expanded";
}
interface Registration {
  renderer: FieldRenderer;
  layout: "inline" | "expanded";
}
const renderers = new SvelteMap<string, Registration>();
function key(codec: CodecIdentity): string {
  return JSON.stringify([codec.name, codec.version]);
}
export function registerRenderer(
  codec: CodecIdentity,
  renderer: FieldRenderer,
  options: RendererRegistrationOptions = {},
): () => void {
  if (!codec.name || !Number.isSafeInteger(codec.version) || codec.version < 1)
    throw Error("Invalid renderer codec identity.");
  if (
    options.layout !== undefined &&
    !["inline", "expanded"].includes(options.layout)
  )
    throw Error("Invalid renderer layout.");
  const id = key(codec);
  if (renderers.has(id)) throw Error("Renderer codec is already registered.");
  const registration = {
    renderer,
    layout: options.layout ?? "expanded",
  } as const;
  renderers.set(id, registration);
  return () => {
    if (renderers.get(id) === registration) renderers.delete(id);
  };
}
export function findRenderer(codec?: CodecIdentity): FieldRenderer | undefined {
  return codec
    ? (renderers.get(key(codec))?.renderer ??
        (semanticKind(codec) ? SemanticField : undefined))
    : undefined;
}

export function rendererLayout(
  codec?: CodecIdentity,
): "inline" | "expanded" | undefined {
  return codec ? renderers.get(key(codec))?.layout : undefined;
}
