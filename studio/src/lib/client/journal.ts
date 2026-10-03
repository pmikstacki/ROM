import { projected, record, text, unsigned } from "./validation.ts";
import type { WireObject, WireValue } from "./types.ts";
export function journalBatch(
  value: WireValue,
  kind: string,
  after: WireObject | null,
  maxRows: number,
): WireObject {
  const o = record(value),
    cursor = record(o.cursor),
    generation = text(cursor.generation),
    position = unsigned(cursor.position);
  if (text(cursor.kind) !== kind) throw new Error("journal identity mismatch");
  let previous = 0n;
  if (after) {
    if (text(after.kind) !== kind) throw new Error("journal identity mismatch");
    if (text(after.generation) !== generation)
      throw new Error("journal generation mismatch");
    previous = unsigned(after.position);
    if (position < previous) throw new Error("journal position regressed");
  }
  if (!Array.isArray(o.events) || o.events.length > maxRows)
    throw new Error("journal events limit");
  const events = o.events.map((value) => {
    const e = record(value),
      at = unsigned(e.position);
    if (at <= previous || at > position)
      throw new Error("journal position order");
    previous = at;
    return { position: at, view: projected(e.view, kind) };
  });
  return {
    cursor: { generation, kind, position },
    events,
  } as unknown as WireObject;
}
