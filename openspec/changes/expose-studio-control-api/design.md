# Design: public Studio controls

The intended user composes an application from ROM controls using public package imports.
The consumer must not configure private source aliases or patch vendor code.

## Selected boundary

Keep `studio/src/index.ts` as a facade. Add `studio/src/controls.ts` as the shared control facade.
The root facade reexports those controls. The independent consumer uses the controls-only `rom-studio/controls` entry.
Existing root App, client and renderer paths stay stable. Browser session APIs require a separate decision.

Controls-only consumption avoids compiling the application shell. Its existing private aliases are outside this change.
The supported controls retain their current native and bits-ui behavior. No wrapper homogenizes distinct value guarantees.

## Alternatives

Root-only exports are small but force independent consumers to resolve private shell dependencies.
A compiled library build would remove aliases but introduces packaging and component-type work beyond these source-native controls.
A controls-only source entry preserves inferred Svelte props and allows narrow relative imports inside the shared controls.

## Existing prop contracts

Input and Textarea bind value and ref. Textarea defaults to content sizing; compact composers can apply fixed sizing.
NativeSelect binds value and ref on its select. Its class applies to the wrapper, identified by native-select-wrapper.
NativeSelectOption and NativeSelectOptGroup bind native element refs and accept native attributes.
Checkbox binds checked, indeterminate and ref. Slider binds its existing discriminated single or multiple value representation and ref.
Button and Label retain their existing ref, class and native or bits-ui props.
All exposed controls retain existing data-slot selectors. Classes target their documented host instead of a uniform invented host.

## Packaging and evidence

The independent fixture copies producer sources and package metadata into an isolated temporary dependency directory.
It resolves public package names through exports, without rom-studio or $lib source aliases.
It uses one installed dependency graph for local verification, with no claim of fresh offline installation acceptance.
Its stylesheet uses public rom-studio/styles and the existing Svelte/Tailwind toolchain.
Browser tests cover bound values, actual element refs, wrapper/control styles, disabled and invalid states, keyboard input and compact layouts.
Producer-owned public control imports may require narrow relative-path changes. Package export metadata needs coordinator ownership approval.

## Independent admission correction

The source stylesheet declares shadcn-svelte and tailwindcss as runtime dependencies because it imports their CSS.
The consumer declares only ROM, Svelte, and selected compiler/type-check/browser tools. Its lock is independent of the producer lock.
Explicit bootstrap produces a candidate lock. Ordinary verification uses the reviewed lock with npm ci.
Runtime source metadata, installed source, realized versions, and frozen lock identity are checked before acceptance.

Evidence output is reserved exclusively before archive or log writes. Reused paths fail without changing historical results.
Authoring mode archives the current checkout and is labeled separately.
Release mode selects a provided Studio archive or the Studio directory from a verified extracted artifact.
Release admission requires physical installation, matching source, locked replay, and actual successful Chromium/WebKit outcomes.
Dependency copies, bootstrap runs, and incomplete browser evidence cannot satisfy that gate.

Candidate archive admission uses the existing shared release extractor after stricter candidate-specific validation.
The archive has one fixed prefix and selected package/source/license members. Member count and compressed/expanded/logical bytes have explicit bounds.
No raw supplied archive is extracted before its paths, links, members, and budgets are admitted.
