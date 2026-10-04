# Actual provider and native maintenance journey

The focused test passed on SQLite and redb. It connects actual OIDC login, generic HTTP mutation, backup, restore, schema migration, disk reopen and fresh login.

This closes a specific acceptance gap. The earlier native maintenance test used synthetic signed identity evidence. Separate browser restart tests did not run backup and migration in the same flow.

## Executed flow

The test uses the existing `oidc-provider` fixture. The fixture issues an authorization code and signed ID token through real HTTP endpoints. The host exchanges the code, verifies the token and binds current managed identity Resources.

The browser helper follows real redirects and submits login and consent forms. It holds cookies by origin. It does not inject an `Actor` or a signed proof.

| Step | Executed assertion |
| --- | --- |
| Initial login | Current managed user is authenticated through the real provider. |
| Generic mutation | HTTP creates one version 1 Resource at revision 1. The stored boolean remains `false`. |
| Live-owner negative control | Migration fails while the source has an active native owner. No destination appears. Counts remain unchanged. |
| Graceful stop | The actual serving task drains and stops within two seconds. Its runtime and database owner are dropped. |
| Backup and restore | A native archive restores into a fresh destination. Resource, receipt, event and effect counts match. |
| Schema migration | Version 1 `done` becomes version 2 `completed`. Migration writes a fresh destination. Original source bytes remain unchanged. |
| Disk reopen | The migration return handle is dropped. A new adapter opens the migrated destination. |
| Historical event | A new backup contains exactly one converted event for the migrated Resource. Its value is `{"completed":false}`. |
| Stale browser negative control | The old cookie is unauthenticated after restart. Its mutation request returns `401`. Counts remain unchanged. |
| Fresh actual login | A new browser completes the real provider flow. The session has the same managed user and a new generation. |
| Durable replay | The original version 1 request returns the converted version 2 value at revision 1. It creates no extra receipt, event or effect. |
| Current identity denial | Disabling the current identity link makes replay return `401` with `{"error":"denied"}`. No further writes occur. |

The provider process remains alive during native maintenance. The host restarts on the same callback origin. No browser session or `Actor` is restored from backup.

## Source

The test is `maintenance::real_provider_backup_migration_reopen_requires_fresh_session_and_current_authority` in [maintenance.rs](../../crates/rom-studio-host/tests/support/maintenance.rs).

It reuses the provider process, browser cookie handling and authorization helpers in [human.rs](../../crates/rom-studio-host/tests/human.rs). Typed test Resources use the same native registration and generic HTTP path before and after migration.

The migration declaration uses `ResourceMigration::new`, `MigrationPlan::new` and `Definition::replay_from`. Native adapters execute `backup_to`, `restore_from` and `migrate_from`. No product API changed.

## First failure and correction

The first run failed at the final identity revocation assertion. The test expected `403`, but the host returned `401`.

Source inspection confirmed the host contract. `protect` calls `authentication::resolve` before the generic HTTP handler. A disabled identity link prevents current identity binding. `authentication::failure` maps that failure to `401` and `{"error":"denied"}`.

Only the test expectation changed. This was a test-contract error, not a product defect. The first failing log remains available. The executed live-owner and stale-cookie negative controls remain in the passing test.

## Verification

The parent authorized one focused compile and one cached rerun. Both used the shared target, one Cargo job and the default debug profile. `CARGO_INCREMENTAL=0` was set. No `RUSTFLAGS` or profile override was used.

Before each command and each `rustc` launch, the capacity checkpoint checked the 92 GiB floor. The container copy has the same SHA-256 as the host script:

```text
3f1fda0065c1a57e6ec0a8273ad1ba2a0793371698b4c5ae16b9c883d1ee16ec
```

The exact Cargo command was:

```sh
cargo test --locked --offline -p rom-studio-host --test human \
  maintenance::real_provider_backup_migration_reopen_requires_fresh_session_and_current_authority \
  -- --exact --nocapture
```

The final run passed one test function, which executed both database variants. It completed in 0.62 seconds after a 1.92-second cached compile. Scoped `rustfmt --check` also passed.

| Identity | Value |
| --- | --- |
| Base revision | `cc05983b506873c5d655254477a63bbe946f77e4` |
| `human.rs` SHA-256 | `d98c5007b93357dec22cc90d3a12d0a129e0d6948e7a7864fa9824ee3d3038ed` |
| `maintenance.rs` SHA-256 | `96cdb876c53c06fcb688f8a069ad6e81fc8d0b132e98dd3b94142ffad1f11969` |
| `Cargo.lock` SHA-256 | `f288709d12f8c7adc39a5d72cb7253a9d86afa990dd94226724e39262500d992` |
| Compiler | Rust 1.99.0, commit `b940084d7eb6a299eb4bfeb8e34901bc051e7ac4` |
| Cargo | 1.99.0 |
| Provider runtime | Node 22.16.0 |

Raw logs:

- [First failing run](evidence/rom-0.0.2/task4/maintenance-provider-focused.log).
- [Final passing run](evidence/rom-0.0.2/task4/maintenance-provider-final.log).

The first compile rebuilt the host-test feature graph, including Tokio and HTTP dependencies. It completed before the attempted stop found a live Cargo process. Its target allocation increased by 511,262,720 bytes. This was reported immediately to the parent.

Final target allocation was 87,017,807,872 bytes. Final root free space was 105,836,494,848 bytes. The full release producer remained deferred.

Final scratch archives and databases were retained inside `rom-dev`:

```text
/tmp/rom-host-maintenance-2192919-false-1791078370857677189
/tmp/rom-host-maintenance-2192919-true-1791078371173415709
```

The first failing SQLite scratch was also retained:

```text
/tmp/rom-host-maintenance-2192829-false-1791078298105888281
```

These scratch paths contain public test identity data. The provider secret is a fixed synthetic fixture credential. It is not stored in the native archive.

## Limits

This test uses HTTP browser helpers, not a rendered Chromium or WebKit Studio interface. Existing rendered-browser gates remain separate requirements.

The test validates converted durable history through the native backup archive. It does not test journal cursor recovery through the browser transport.

The provider uses its documented development-only in-memory adapter. Provider durability and real deployment provider operations are outside this proof.

This run is not a complete release verifier. The parent must include the new test and dependency edge in the combined gate and source inventory.
