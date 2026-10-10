# Portable Studio App candidate review

Date: 2026-10-07. Scope: installed public root App and supported custom-renderer composition.

The installed consumer first failed because the App graph imported a private `$lib` alias.
The root coordinator changed 335 import specifiers across 293 maintained source files to relative imports.
The mechanical comparison found no other changes within those files.
Existing relative public controls and recovery modules were outside this mechanical comparison.

The candidate now builds and type-checks from a physical installation with its frozen consumer lock.
The consumer imports App and renderer registration from the public package entry.
It visits Resources, Settings, and Work at desktop and mobile widths. An explicitly registered custom codec supplies its field control.
Backend responses are simulated in this fixture. These cases do not establish real persistence or authority behavior.

The first browser run retained two mobile navigation failures.
The test treated a closing mobile drawer as visible, then waited for a button after the drawer detached.
The correction waits for drawer closure, opens navigation, and selects the next screen. It uses no fixed sleep or forced click.
The corrected fixture passed twelve cases across Chromium and WebKit.

The maintained component suite passed 221 cases and skipped one case. Studio unit tests passed 184 cases.
Evidence: `/var/tmp/rom-010-root-entry-evidence/` and `/var/tmp/rom-portable-root-browser-fixed-20261007/`.
The import proof is `/var/tmp/rom-010-root-entry-evidence/mechanical-diff-proof.json`.
The initial host command used the wrong working directory; its failure remains recorded separately from browser behavior.

These results concern a dirty checkout candidate, not independently verified clean release source.
The CLI's extracted-source label does not remove that limitation. SSR support is not established for the root App.
Full local verification, branch review, clean-source release admission, and real external-consumer migration remain open.
