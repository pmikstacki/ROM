import { parseWire, stringifyWire } from "rom-studio/client";
import type { ProjectedView, WireValue, WireObject } from "rom-studio/client";
import { validateLayout } from "rom-studio/ui";
import type { LayoutItem, LayoutOptions } from "rom-studio/ui";

export const kinds = [
  "equipment",
  "inspections",
  "work-orders",
  "portal-settings",
] as const;
export type PortalKind = (typeof kinds)[number];
export const layoutLabels = {
  left: "Move left",
  right: "Move right",
  up: "Move up",
  down: "Move down",
  wider: "Make wider",
  narrower: "Make narrower",
  taller: "Make taller",
  shorter: "Make shorter",
  show: "Show",
  hide: "Hide",
  invalid: "Invalid stored layout",
  failed: "Settings rejected",
  unknown: "Settings outcome unknown",
};
export const panelLabels: Record<string, string> = {
  history: "Inspection history",
  equipment: "Equipment",
  work: "Work orders",
};
export function layoutOptions(columns: bigint | number): LayoutOptions {
  const count = Number(columns);
  if (!Number.isSafeInteger(count) || count < 1 || count > 4)
    throw Error("Invalid host columns");
  return {
    columns: count,
    maxRows: 8,
    maxItems: 3,
    catalog: Object.keys(panelLabels).map((id) => ({
      id,
      minWidth: 1,
      maxWidth: 4,
      minHeight: 1,
      maxHeight: 4,
    })),
  };
}
export function settingsLayout(
  view: ProjectedView | undefined,
): { options: LayoutOptions; layout: readonly LayoutItem[] } | null {
  if (
    !view?.value ||
    typeof view.value.layout !== "string" ||
    view.value.layout.length > 65536
  )
    return null;
  const options = layoutOptions(view.value.columns as bigint);
  const result = validateLayout(JSON.parse(view.value.layout), options);
  if (!result.valid) return null;
  return { options, layout: result.layout };
}
export function cloneView(view: ProjectedView): ProjectedView {
  return parseWire(
    stringifyWire(view as unknown as WireValue, 65536),
    65536,
  ) as unknown as ProjectedView;
}
export function field(view: ProjectedView | undefined, name: string): string {
  return typeof view?.value?.[name] === "string"
    ? (view.value[name] as string)
    : "";
}
export function replacement(
  view: ProjectedView,
  fields: WireObject,
): WireObject {
  if (!view.value) throw Error("Resource unavailable");
  return { ...view.value, ...fields };
}
