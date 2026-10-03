# Beskid configuration hierarchy: source findings for ROM

Date: 2026-10-02. This report covers source inspection only. No Beskid build, configuration experiment, or reload test was executed. This note treats the owner's premise as fixed: managed entities including users, identity-provider configuration and application settings are Resources. It proposes how configuration providers can feed those Resources without treating executable adapters as stored domain entities.

## Evidence and scope

Read the existing clean local checkouts `/tmp/rom-beskid-quality` at root commit `abfe7d6bf1628db1be75245693b699394eb1a23c` and `/tmp/rom-beskid-compiler-quality` at compiler commit `44a07aed5a13d41d57853445167c0f091cd10a7e`, matching the [earlier audit](beskid-quality-baseline.md). Also read the root's pinned BSOL submodule sources at `f227b51ec3352fb449a8b9ffc3d1f07a74159bb9` through GitHub. No conversation transcript was used.

There is **not one universal Beskid configuration hierarchy established by these sources**. The implemented shell settings loader has a concrete two-file overlay. Board layouts use a different fallback rule. Package-server environment configuration is separate. BSOL supplies document syntax/schema facilities; a profile named `configuration.v2` does not itself establish deployment precedence or live reload.

## Exact shell settings behavior

Effective precedence, lowest to highest, is:

1. Registered setting defaults.
2. User file `~/.beskid/config/tools.bsol`.
3. One selected project **or** workspace root's `.beskid/tools.bsol`.

The implementation loads user values, then selected-scope values. Only afterward does it fill absent keys with defaults. It does not merge a chain of every ancestor directory, nor layer project settings on top of workspace settings. The `is_file()` and `if let Ok(parsed)` checks skip missing/unreadable/invalid files. The caller receives an effective config without those errors. Paths use `dirs::home_dir()` with `.` fallback; this loader has no environment-variable or CLI-value overlay. These are observations about this loader, not all Beskid commands. [Load/merge/path source](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_tools/src/shell/settings.rs#L134).

The merge key is `(tool_id, key)` and the value is a string. Later values replace earlier values at that key; unrelated keys remain. Lowering repeated setting blocks inserts into the same map, so a later repeated logical key wins. This is shallow keyed replacement, with no recursive object merge, list append, deletion marker, null semantics or provider identity. Nonzero incoming configuration versions overwrite the previous version. The loader does not enforce a version of one. Output sorts tool/key pairs before serialization. [Lowering and emission](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_tools/src/shell/settings.rs#L218).

### Scope discovery is selection, not layer merging

`ShellScope::resolve` tries these categories in order:

1. A workspace manifest in the starting directory or its ancestors.
2. A project manifest in the starting directory or its ancestors.
3. A descendant workspace manifest.
4. A descendant project manifest.
5. User scope.

Consequently, an ancestor workspace can win even when the starting directory contains a project. Within an ancestor category the first found manifest is nearest. Descendant search is breadth-first with depth limit six and skips `.git`, `.beskid`, `.cargo`, `.generated`, `build`, `dist`, `node_modules`, `target` and `vendor`. The selection helper chooses the shallowest result. Equal-depth ties are not explicitly sorted by path. Per-directory discovery reports multiple manifests as ambiguity, but these convenience search functions only accept `Ok(Some(...))`, thereby skipping such errors. These details are visible in code. They were not verified through execution of the discovery functions. [Scope resolver](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_tools/src/shell/scope.rs#L22), [manifest discovery](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_analysis/src/projects/discovery.rs).

### Schema, generated settings UI and provenance

The `tools.config.v1` profile requires a config block and allows many settings blocks with quoted tool ID, key and value. Separately, Rust `ToolSettingDescriptor` carries label, description, kind (`Bool`, `U32`, `Quoted`) and default; pages group these descriptors by tool. Duplicate page IDs are silently ignored by the registry. Parsing the file does not consult the descriptor registry to reject unknown setting keys or validate a string as its declared boolean/integer kind. The settings widget renders from those descriptors. [BSOL profile](https://github.com/Cyber-Nomad-Collective/beskid_bsol/blob/f227b51ec3352fb449a8b9ffc3d1f07a74159bb9/schemas/tools.config.v1.bsol), [descriptor registry](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_tools/src/shell/settings.rs#L10), [settings widget](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_tools/src/shell/widgets/settings.rs).

The effective `ToolsConfig` retains only version and flattened key/value map. It does not retain which layer supplied a value, shadowed alternatives, file span or whether the value was defaulted. Saving writes the entire effective config to the selected scope file, or to the user file in user scope. Defaults and inherited values have already entered that map. Thus, a save can materialize them as explicit overrides and prevent later parent changes from taking effect. The writer uses ordinary `fs::write`. No revision check or atomic replacement protocol is present here. This is a concrete caution for a multi-client server design. [Config storage/writer](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_tools/src/shell/settings.rs#L134), [widget save](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_tools/src/shell/widgets/settings.rs#L90).

### Reload evidence is limited

The host loads settings at construction and reloads config/key bindings when switching scope. When its scope label changes, the settings widget loads. Reset restores its saved in-memory snapshot. Saving synchronizes shortcut bindings to the host. These are specific interactive refresh paths. No file-watcher-driven general settings reload, transactional multi-provider activation, or guarantee that every registered setting affects a running subsystem is demonstrated by this path. A searchable descriptor is not evidence that its consumer uses it. [Host initialization](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_tools/src/shell/host/app.rs#L68), [scope reload](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_tools/src/shell/host/actions.rs#L238), [widget load/reset](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_tools/src/shell/widgets/settings.rs#L52).

## Other Beskid configuration paths must stay distinct

- **Board layout:** if the selected scope's complete board document is present, read it. Otherwise, use the embedded default. Project/workspace layout loading does not merge the user board. This is whole-document fallback, not settings overlay. [Layout loader](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_tools/src/shell/layout/load.rs).
- **General BSOL configuration profiles:** `configuration.v2` admits config metadata, arbitrary module blocks and key/value options. Profile composition concerns schema composition; it does not define provider precedence, effective-value provenance or server activation. The shared language deliberately leaves application semantics to its consumer. [Profile](https://github.com/Cyber-Nomad-Collective/beskid_bsol/blob/f227b51ec3352fb449a8b9ffc3d1f07a74159bb9/schemas/configuration.v2.bsol), [BSOL architecture](https://github.com/Cyber-Nomad-Collective/beskid_bsol/blob/f227b51ec3352fb449a8b9ffc3d1f07a74159bb9/README.md).
- **Package server:** `PckgServerConfig::from_environment()` builds a typed configuration from defaults and named environment variables, with builder methods and startup checks. It is not wired through the shell's user/project files. Operational configuration also validates dependent fields before binding adapters. Neither example proves generic reload. [Server configuration](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_pckg_server/src/server/config.rs), [operational validation](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_pckg_operations/src/config.rs).

## What ROM should carry forward

Borrow **registered typed descriptors, deterministic layering, generated settings controls and explicit scope**. Improve the semantics required by a long-running service. The following are ROM recommendations, not claims about implemented Beskid or approved final ROM precedence:

| Concern | Recommended ROM contract |
| --- | --- |
| Resource model | `User`, `IdentityProviderConfig`, `ApplicationSettings` and other managed configuration are ordinary compiled Resource kinds, with identity, policy, fields and permitted actions. Built-in origin does not create a parallel entity system. |
| Provider input | File/env/remote providers yield typed partial Resource values plus source identity, scope, version and presence information. Core owns validation and resolution; provider formats remain adapters. |
| Hierarchy | Host declares a deterministic ordered set of sources/scopes. Define permitted tenant/application/deployment overrides explicitly; do not infer trust or precedence from the server's current directory. A universal env-over-CLI-over-file order is not established by the Beskid evidence. |
| Merge | Merge by Resource identity and declared field semantics. Omission inherits; null is an explicit value where valid; removing an override is a distinct operation. Lists replace unless a field explicitly defines keyed merging. Reject ambiguous same-priority duplicates. |
| Provenance | Preserve each effective field's source and revision, inherited/default status and permitted write target; show a redacted explanation in Studio. Never require copying the whole effective record to change one override. |
| Invalid sources | Distinguish absent optional source from unavailable required source and invalid supplied data. Validate the full candidate snapshot before activation; reject invalid auth settings explicitly. |
| Editing | A read-only deployment source stays read-only. Studio can target a designated writable provider or authorized overlay, with revision checks, rather than silently writing a shadow copy. |
| Activation | Separate accepted desired Resource state from applied runtime generation/status. Mark which changes apply live, need reconnection or require restart. A persisted edit does not prove the adapter accepted it. |

These supporting values, statuses and provider interfaces do not need new first-class domain entity categories. If a source or activation itself needs managed identity/history, represent it as another Resource kind. The central rule is consistent managed semantics, not that every process object must become persistent resource state.

## Configuration Resources versus executable adapters

An identity-provider Resource can store provider kind, issuer, client identity, secret reference, enabled state and allowed policy configuration. Its implementation is a host-registered Rust adapter with executable validation/network behavior. The Resource selects and configures an available implementation. It cannot turn arbitrary stored text into a trusted plugin. Likewise, an application-settings Resource holds managed data, while the file configuration provider is infrastructure that supplies or persists that data. Adapters can publish status through Resources and react to accepted configuration changes without becoming interchangeable with their configuration records.

Core therefore still owns capability admission, current authorization and the action path for configuration changes. Built-in Resources may expose narrower operations: secret replacement or provider activation can be dedicated actions, with redacted reads and audit events. A generic editor must not make all configuration fields openly readable/writable. This fits the owner's single-Resource premise and the [existing adapter requirements](../../openspec/changes/design-capability-adapters/specs/capability-adapters/spec.md).

Bootstrap needs an explicit boundary: the host must have enough external configuration to construct initial providers, persistence and trusted policy before normal Resource operations can run. Where possible, decode that bootstrap input with the same compiled configuration descriptors. Then expose the effective managed configuration through Resources. Avoid a circular rule requiring the database-backed configuration Resource to select the database needed to load it. Later provider changes need a documented handover/restart protocol, not an assumption that reassigning a record reconfigures live connections atomically.

## Smallest useful acceptance

Declare built-in `ApplicationSettings` and `IdentityProviderConfig` Resources. Load defaults plus one deployment file and one explicit writable override source. Verify precedence, partial inheritance, null versus omitted, override removal, source provenance, redaction and conflicting edits. Studio must show the same effective values as embedded resource reads, and explain which source can be changed.

Inject an invalid provider update and retain the previous active generation with an explicit failure status. Prove one chosen live-change path and mark restart-only fields honestly. Restart and re-resolve the same inputs; confirm no accidental promotion of inherited values into the override source. File persistence, resource commit and external adapter activation are separate failure boundaries and must not be advertised as one atomic transaction without implementation evidence.

This is the transferable Beskid lesson: a small registry plus scoped composition can drive useful administration. ROM needs stronger validation, provenance, concurrency and activation semantics because its configuration controls a persistent, authorized server rather than only a developer shell.
