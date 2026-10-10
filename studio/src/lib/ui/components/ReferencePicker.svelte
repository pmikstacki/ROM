<script lang="ts">
  import { ReferencePicker as SharedReferencePicker } from "rom-ui/ui/components";
  import type { ReferencePickerMessages } from "rom-ui/ui/components";
  import type { ReferenceLookup } from "./reference-lookup.ts";
  import type { WireValue } from "../../client/types.ts";
  import { normalizeValue } from "../../client/codec.ts";
  let { kind, lookup, authorityToken, messages = {}, value, label, path = label,
    onchange, onerror = () => {}, readonly = false, showLabel = true,
  }: { kind: string; lookup?: ReferenceLookup; authorityToken: unknown;
    messages?: Partial<ReferencePickerMessages>; value: WireValue; label: string;
    path?: string; onchange: (value: WireValue) => void;
    onerror?: (message: string) => void; readonly?: boolean; showLabel?: boolean;
  } = $props();
  function normalizeId(text: string, referenceKind: string, fieldPath: string): string {
    const normalized = normalizeValue({ type: "reference", value: { kind: referenceKind } }, text, fieldPath);
    if (typeof normalized !== "string") throw TypeError("Invalid reference ID.");
    return normalized;
  }
  function change(next: unknown) {
    if (typeof next !== "string") throw TypeError("Invalid reference ID.");
    onchange(next);
  }
</script>
<SharedReferencePicker {kind} {normalizeId} {lookup} {authorityToken} {messages}
  {value} {label} {path} onchange={change} {onerror} {readonly} {showLabel} />
