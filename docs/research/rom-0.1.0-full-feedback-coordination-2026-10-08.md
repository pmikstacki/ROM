# Full usability feedback coordination

Date: 2026-10-08. This document assigns verification ownership; it does not grant release acceptance.

The [intake](rom-0.1.0-usability-intake.json) is the source of feedback references and acceptance conditions.
The historical audit retains 47 references across 33 groups. Two groups contain withdrawn requests.
The fresh original-thread read retains update marker 1791445000 and consumer-reported commit 86dca6f.
A recent-page read cannot establish complete historical coverage.

## Ownership

| Group | Feedback | Verification owner | Boundary |
| --- | --- | --- | --- |
| AP-UX-001 | Public shared controls | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-002 | Control styling ownership | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-003 | Compact composer and form bindings | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-004 | Autosave and uncertain mutation recovery | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-005 | Safe navigation and drafts | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-006 | Session refresh and view sequencing | Identity + UI | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-007 | Actual work stages and durable progress | AI + UI | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-008 | Immediate pending queue and resume | AI + UI | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-009 | Long results and composer geometry | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-010 | Respect manual scroll | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-011 | Responsive panels, focus and reduced motion | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-012 | Localization and structured error messages | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-013 | Stale cache and recoverable observation | Identity + UI | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-014 | Useful conversations and grounding | AI + UI | Domain behavior stays in the app; extract reusable mechanisms |
| AP-UX-015 | Lazy topic research and deduplication | AI + UI | Domain behavior stays in the app; extract reusable mechanisms |
| AP-UX-016 | Domain tool transparency | AI + UI | Domain behavior stays in the app; extract reusable mechanisms |
| AP-UX-017 | Selectable cards and reactive computation | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-018 | Dashboard widget layout and settings drawer | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-019 | Chart terminology and dynamic label spacing | Consumer + coordinator | Domain behavior stays in the app; extract reusable mechanisms |
| AP-UX-020 | Autocomplete with explicit manual fallback | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-021 | Admin-only navigation and guest workflows | Identity + UI | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-022 | Readable saved-history list | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-023 | Source links retain application context | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-024 | Fresh execution versus retained result | AI + UI | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-025 | PDF/export from actual selected snapshot | UI + coordinator | Verify public implementation, regression, installed consumer, and application adoption |
| AP-UX-026 | Private observations/profile | Consumer + coordinator | Domain behavior stays in the app; extract reusable mechanisms |
| AP-UX-027 | Model provider failover policy | AI + UI | App chooses provider policy; ROM enforces bounded routing |
| AP-UX-028 | Theme and domain brand | Consumer + coordinator | Domain behavior stays in the app; extract reusable mechanisms |
| AP-UX-029 | Daily questions and relationship-aware readings | Consumer + coordinator | Domain behavior stays in the app; extract reusable mechanisms |
| AP-UX-030 | Video frame-rate request withdrawn | No implementation | Withdrawn; retain history |
| AP-UX-031 | Date timeline and selected temporal context | Consumer + coordinator | Domain behavior stays in the app; extract reusable mechanisms |
| AP-UX-032 | Music and sound request withdrawn | No implementation | Withdrawn; retain history |
| AP-UX-033 | Guest invitation and optional account save | Consumer + coordinator | App owns invitations; ROM verifies public access and optional persistence |

## Acceptance record

For each active group, retain these separate evidence states:

1. Source implementation and its public entry point.
2. Executed regression, including failure conditions and source identity.
3. Installed consumer result without private imports or source aliases.
4. Original application result, with consumer revision and installed ROM identity.
5. Human usability observations where the acceptance condition needs them.

A passing generic fixture does not complete original application acceptance.
Domain-specific feedback remains tracked even when its behavior belongs to Astral Plane.
Keep withdrawn requests withdrawn. Preserve failed tests and earlier evidence.

The coordinator sent the complete-feedback requirement to the active UI, identity, and AI workers.
Shared handoff: [ROM coordination](rom-0.1.0-coordination.md) and /root/astral-plane/docs/rom-0.1.0-coordination.md.
The environment has no direct message tool for the original chat. Its acknowledgement is not established.

## Current increment

The corrected connected portal passed 60 actual browser cases across SQLite/redb and Chromium/WebKit.
Independent review accepted its WorkOrder recovery and proxy bind cleanup corrections for the witnessed candidate.
Evidence: /var/tmp/rom-010-portal-recovery-matrix-witness.json.
The public catalog passed two actual HTTP tests and two typed/live projection tests across the database adapters.
The complete maintenance consumer passed 14 tests and all-target Clippy.
Connected guest browser verification remains in progress. Final combined verification and release admission remain open.
