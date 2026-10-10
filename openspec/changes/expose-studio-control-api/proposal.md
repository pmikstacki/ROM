# Public Studio controls

## Why

Astral Plane imports controls that already exist in Studio through a vendor patch.
The producer public entry exposes only Input. Control modules also depend on a private build alias.

## What changes

Expose Input, Button, Textarea, NativeSelect, NativeSelectOption, NativeSelectOptGroup, Checkbox, Slider and Label through maintained public entries.
Provide a controls-only entry for independent Svelte consumers. Preserve existing root exports and control behavior.
Document and test the existing class, ref and value contracts through a copied package consumer.
Evaluate stylesheet resolution and browser behavior without private consumer aliases.

## Impact

No new UI dependency or browser auth export is included.
Runtime notice ownership remains a separate R3 change. Release acceptance still requires the full verifier and independent artifacts.
