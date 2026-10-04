# Studio presentation verification

Date: 2026-10-04. Status: the presentation checks passed. Full release acceptance remains incomplete.

## Implemented scope

Studio now uses the official responsive Sidebar and controlled shadcn inputs. Resource descriptors still select navigation, fields, queries and actions.
The registry installation contains 59 component families. The current inventory is studio/src/lib/components/ui/registry-provenance.json.
The initial four-family inventory remains historical evidence in studio/upstream-components.json.

SelectAdapter uses internal option tokens. It preserves empty and arbitrary domain strings without treating them as placeholders.
CheckboxAdapter associates each control with its label. FieldIntent still distinguishes omit, value, null and remove.
ResourceDetails remains one component across layout changes. A newer revision retains the draft and blocks stale submission.
The shell keeps pending unknown mutations outside the keyed Resource page. Navigation changes do not create another retry identity.

## Keyboard correction

An invisible Sidebar tooltip intercepted Escape after focus entered mobile navigation. Escape closed the tooltip but left the Sheet open.
Studio now creates tooltips only for collapsed desktop navigation. The mobile Sheet closes and returns focus to its trigger.
The copied Sidebar also forwards an optional close-focus callback. Provenance records this adaptation separately from the application tooltip correction.
The regression checks assert that the dialog closes, focus returns, another Resource opens, and the document has no horizontal overflow.

## Executed checks

| Check | Result | External evidence filename |
| --- | --- | --- |
| Svelte diagnostics | 0 errors, 0 warnings | ROM-shadcn-final-source-checks.json |
| Client unit tests | 70 passed | ROM-shadcn-final-source-checks.json |
| Controller component tests | 22 passed | ROM-shadcn-node-component-checks.json |
| Notice collector tests | 7 passed | ROM-shadcn-final-source-checks.json |
| Component browser tests | 50 passed, Chromium and WebKit | ROM-shadcn-tooltip-focus-verification.json |

Evidence files are in /root/ipi/research/disk-coordination-2026-10-04/.
The final browser command was npm run test:components. It built the component assets before running both engines.
Browser runs used the inspected font configuration and software Mesa runtime. Shader caching was disabled for the final run.

The production build also completed. Its inventory retained copied controls, the responsive hook, and imported CSS package notices.
CSS source ownership does not claim a complete transformation source map. License text remains unchanged.

## Screenshots and limits

Actual native-host screenshots are in /root/ROM-design-screenshots/shadcn-layout/. The capture manifest distinguishes binary and frontend source identities.
The screenshots use the current working-tree frontend and an earlier native binary. They are design evidence, not accepted release artifacts.
The owner will critique these screenshots before another visual redesign. Component provenance does not establish human usability.

The eight complete release gates, extracted artifacts and permanent VPN preview remain pending.
This verification does not claim production readiness or replace native browser acceptance on both persistence adapters.
