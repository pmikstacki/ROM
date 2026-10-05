import type { ResourceDescriptor, ProjectedView } from "../client/types.ts";

/** Resolve only disclosed fields. Presentation never provides authority. */
export function resourceLabel(descriptor: ResourceDescriptor): string {
  return descriptor.presentation?.label || descriptor.kind;
}
export function resourceTitle(
  descriptor: ResourceDescriptor,
  row: ProjectedView,
): string {
  const field = descriptor.presentation?.title_field;
  if (
    field &&
    descriptor.fields.some((candidate) => candidate.name === field)
  ) {
    const value = row.value?.[field];
    if (typeof value === "string" && value.trim()) return value;
  }
  const label = descriptor.presentation?.label || "Resource";
  const id =
    row.key.id.length > 48 ? `${row.key.id.slice(0, 45)}…` : row.key.id;
  return `${label} ${id}`;
}
export interface SettingsSection {
  group: string;
  label: string;
  resources: ResourceDescriptor[];
}
export function settingsSections(
  descriptors: ResourceDescriptor[],
): SettingsSection[] {
  const groups = new Map<string, SettingsSection>();
  for (const resource of descriptors) {
    const settings = resource.presentation?.settings;
    if (!settings) continue;
    let section = groups.get(settings.group);
    if (!section) {
      section = { group: settings.group, label: settings.label, resources: [] };
      groups.set(settings.group, section);
    }
    section.resources.push(resource);
  }
  return [...groups.values()].sort((a, b) => a.group.localeCompare(b.group));
}
