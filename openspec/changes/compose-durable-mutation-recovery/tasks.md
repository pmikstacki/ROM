## Design and review

- [x] 1. Inspect client, controller, consumer autosave, durable principal and replay sources.
- [x] 2. Define exact public interfaces, storage ordering, draft states and identity boundaries.
- [x] 3. Prepare failing unit, HTTP and external browser acceptance scenarios in the implementation plan.
- [x] 4. Review and accept the design and plan before product edits.

## Implementation and acceptance

- [x] 5. Write and execute the first failing public-helper tests; record the intended failure.
- [x] 6. Implement the narrow recovery record and controller modules with public facades.
- [x] 7. Test storage failures, principal switches, generations, exact values and uncertain refusals.
- [x] 8. Run lost-acknowledgement and restart acceptance over actual HTTP on SQLite and redb.
- [x] 9. Run the extracted Svelte consumer in Chromium and WebKit on both adapters.
- [x] 10. Add an unrelated consumer composition and navigation/draft example.
- [ ] 11. Run affected checks, the full local verifier and independent artifact acceptance before integration.
- [ ] 12. Record complete release acceptance evidence. Pure-helper evidence and remaining limits are recorded in the implementation plan.
