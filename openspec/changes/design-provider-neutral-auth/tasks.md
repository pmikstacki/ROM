## 1. Validate the proposed design

- [x] 1.1 Research primary protocol and Rust crate sources and record trust boundaries and alternatives.
- [ ] 1.2 Select two provider profiles and demonstrate distinct issuer and human/service mappings in disposable examples.
- [ ] 1.3 Resolve actor validity, field paths, policy snapshot/freshness, safe query behavior, and deleted-resource retry outcomes.
- [ ] 1.4 Validate conceptual interfaces with an in-process caller and a transport adapter before fixing public Rust signatures.
- [ ] 1.5 Specify User as a Resource, verified identity linking/provisioning, first-administrator bootstrap, protected account actions and disablement freshness; test the shared pipeline without a parallel user entity engine.

## 2. Introduce trusted context and authorization

- [ ] 2.1 Implement immutable actor values and explicit trusted host construction; verify forged context and identity-collision scenarios.
- [ ] 2.2 Implement deny-default action/resource/field/read decisions; verify errors, forbidden fields, response projection, and revision conflicts.
- [ ] 2.3 Implement authorization-aware deduplication and safe outcomes; verify revoked access, principal isolation, deletion, and changed-input cases.
- [ ] 2.4 Implement subscription/delivery decisions and service reaction identity; verify revocation, expiry, historical field protection, and no inherited authority.

## 3. Introduce optional verification adapters

- [ ] 3.1 Implement the selected JWT profile with invalid signature/issuer/audience/expiry/type tests and bounded key-rotation recovery.
- [ ] 3.2 Implement introspection with active/resource-binding/freshness checks and unavailable-provider tests.
- [ ] 3.3 Verify no authentication or policy-engine dependency is required by the core library.

## 4. Verify and promote

- [ ] 4.1 Verify every provider-neutral-auth scenario and confirm credentials are absent from logs, stored outcomes, and events.
- [ ] 4.2 Record provider interoperability results and exact supported profiles without claiming arbitrary compatibility.
- [ ] 4.3 Archive and promote only after implementation and conformance evidence exist.

No migration or deletion work applies to this specification-only change. Login/session management and delegated background execution require separate decisions.
