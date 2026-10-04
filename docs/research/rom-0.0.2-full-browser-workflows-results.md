# Connected generic browser workflow results

Date: 2026-10-04.

## Scope

The tests use the real Studio production assets and Rust host. The human provider requires sign-in and consent. Both SQLite and redb run the same generic browser workflow. Each browser creates a Task and an Inventory Resource through descriptor-generated forms. Each workflow reads, patches, filters, sorts and deletes its new Resource.

Task invokes the declared unit action `complete`. Inventory invokes the declared scalar-u64 action `restock`. These are business functions registered through the shared Resource definition. The UI does not add a controller or endpoint for either kind. Inventory starts with `18446744073709551615`, updates to zero, then exercises finite changes and restock. The browser preserves the maximum-u64 value exactly.

The retained native binary is `/var/tmp/rom-studio-inventory-action-acceptance/rom-demo`. Its SHA-256 is `c93a20cfc1d92cc6e3e6f613105bbbcc61468179008fc118fafabb994d1c88f3`. Root built this binary from the accepted demo action source. This test task created no new native build or target. The producer gate still requires a fresh accepted-checkout build.

## Connected accessibility and keyboard checks

The test waits until the Create control is enabled. It focuses that control and activates it with Enter. It edits a field using keyboard selection and text input. Axe scans the connected create forms, live Inventory details and post-delete views. The selected rules cover WCAG 2 A/AA and WCAG 2.1 A/AA. An automated scan is not a full manual accessibility assessment.

An initial scan observed insufficient contrast while a navigation button transitioned between selected and unselected styles. The unchanged button styles pass after current finite document animations finish. The test waits for those animations with a five-hundred-millisecond bound. It does not disable contrast rules, remove controls or change product CSS. The transient failure remains in `label-probe.log`.

Other initial harness failures are retained. Keyboard focus was attempted while Create was disabled during loading. That setup now waits for admission. Query controls have the generated accessible name `Query value value`; the test now uses that exact name. A query change can already stop live observation, so teardown does not click an absent Stop control.

## Evidence

The initial complete workflow passed four cases: Chromium and WebKit on SQLite and redb. See [the browser output](evidence/rom-0.0.2/full-workflows/browser-results.log). The Svelte check reported no errors or warnings. The test records three finite mutation-to-frame samples with a five-cell Inventory table. These local samples include browser-driver, network and frame scheduling. They do not measure isolated renderer CPU time, heap reclamation or a universal latency budget.

The strengthened test uses a second real page in the same browser session for mutations. The first page only observes the live query. Its query-request count must stay unchanged during the three Inventory changes and Task completion. Inventory keeps exactly five cells after every snapshot. Task completion removes only the completed row from the open-task query; an existing open Task must remain visible. Explicit teardown requires the observed network request to terminate.

The measured or scanned page is brought to the foreground before frame and accessibility checks. A retained background-page probe exposed frame throttling and another transitional contrast sample. Foregrounding defines the measurement condition; it does not change the Resource data or suppress an accessibility rule. See [the observer-only run](evidence/rom-0.0.2/full-workflows/observer-only-results.log) and [the retained background probe](evidence/rom-0.0.2/full-workflows/background-frame-probe.log).

All four strengthened cases passed. The observer made zero query requests during each three-snapshot sequence. Every connected axe scan returned no violations under the selected rules after finite transitions settled. Both action forms, both delete flows and explicit stream teardown passed. The final Svelte check reported no errors or warnings.

The two-page samples are not comparable to the earlier single-page timings. Chromium's sequential samples ranged from about 159 to 4151 milliseconds; WebKit's ranged from 136 to 142 milliseconds. Browser page activation and background throttling contribute to these measurements. They establish finite observed frames and bounded DOM size, not a ranking of adapters or renderer performance. No heap-retention or garbage-collection result is claimed.

The observer-page setup initially omitted the Playwright context parameter. That setup failure is retained separately and does not indicate a host failure. Its four owned host processes were terminated with SIGTERM. No other processes, source or evidence were removed.
