import type { CodecIdentity } from "../client/types.ts";
import type { FieldRenderer } from "./types.ts";
import { SvelteMap } from "svelte/reactivity";
const renderers = new SvelteMap<string, FieldRenderer>();
function key(codec: CodecIdentity): string {
  return JSON.stringify([codec.name, codec.version]);
}
export function registerRenderer(
  codec: CodecIdentity,
  renderer: FieldRenderer,
): () => void {
  if (!codec.name || !Number.isSafeInteger(codec.version) || codec.version < 1)
    throw Error("Invalid renderer codec identity.");
  const id = key(codec);
  if (renderers.has(id)) throw Error("Renderer codec is already registered.");
  renderers.set(id, renderer);
  return () => {
    if (renderers.get(id) === renderer) renderers.delete(id);
  };
}
export function findRenderer(codec?: CodecIdentity): FieldRenderer | undefined {
  return codec ? renderers.get(key(codec)) : undefined;
}
