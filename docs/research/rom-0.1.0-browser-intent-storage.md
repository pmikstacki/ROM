# Browser intent storage admission

Date: 2026-10-07. Evidence: current primary-source reading, not an executed browser test. Root owns the store implementation and installed acceptance.

## Primary-source findings

IndexedDB `strict` is a durability hint. Commit checks persistent writes; `complete` follows commit. A successful request alone does not establish transaction completion. Abort reverts transaction changes; commit can fail through quota or other storage errors. [Transaction lifecycle and commit algorithm](https://w3c.github.io/IndexedDB/#transaction-lifecycle), [durability](https://w3c.github.io/IndexedDB/#durability-hint), [commit](https://w3c.github.io/IndexedDB/#commit-a-transaction).

Overlapping read/write transactions are serialized. Keep compare, capacity check and put/delete in one transaction. Enqueue dependent requests within active request callbacks; arbitrary asynchronous work can leave the transaction inactive. [Scheduling](https://w3c.github.io/IndexedDB/#transaction-scheduling), [lifecycle](https://w3c.github.io/IndexedDB/#transaction-lifecycle).

Open requests can wait for other connections after `versionchange` and report `blocked`. The open API has no abort method. Close late successful connections after a host timeout. Closing a connection waits for existing transactions; it does not itself abort them. [Open request](https://w3c.github.io/IndexedDB/#open-requests), [closing](https://w3c.github.io/IndexedDB/#closing-a-database-connection).

Database partitioning does not separate applications by URL directory. Sensitive records require an explicit host privacy policy. [Cross-directory access](https://w3c.github.io/IndexedDB/#cross-directory-attacks), [sensitive data](https://w3c.github.io/IndexedDB/#sensitivity-of-data).

The Storage Standard permits removal of best-effort buckets under storage pressure. Persistence requires permission and still permits user-directed clearing. Usage/quota estimates are approximate. A strict transaction does not request persistence or prevent eviction. [Persistence permission](https://storage.spec.whatwg.org/#persistence), [management](https://storage.spec.whatwg.org/#management), [quota](https://storage.spec.whatwg.org/#usage-and-quota).

The inspected WPT complete-event case asserts upgrade completion precedes open success and cursor results. It does not establish crash durability, quota handling or ROM CAS behavior. This investigation did not run WPT. [WPT source](https://github.com/web-platform-tests/wpt/blob/master/IndexedDB/idbtransaction-oncomplete.any.js).

## Selected ROM contract

Root selected `openIndexedDbIntentStore({ name, maxBytes, maxSlots, timeoutMs?, factory? })`, returning `PendingIntentStore` plus `close()`. Name and bounds are mandatory. No recovery helper selects this store implicitly.

The version 1 database owns a `records` store. Store exact `{ version, payload }` strings. Bound UTF-8 payload plus metadata and slot count. Capacity failure must reject without deleting another slot. Compare and mutation share one strict transaction. Resolve success only on transaction completion.

Host-selected slots distinguish intent and editor records. They can share an explicitly selected database or use separate database names. Storage does not grant authority. Original principal/target binding and current server authorization remain separate checks. Exclude credentials, CSRF tokens and session secrets.

Use bounded open and operation deadlines. After timeout, retain event handlers that close a late successful connection. Handle blocked upgrades, version changes, explicit close and transaction abort. Fail closed if strict durability is unavailable; do not silently use relaxed writes.

## Acceptance and limits

Required acceptance includes competing CAS writers, stale versions, close/reopen, exact bigint wire payload, capacity rejection and capacity release after deletion. Inject request success followed by abort: no durability acknowledgement may escape. Add blocked-open timeout, late success cleanup, versionchange shutdown and storage/quota failure cases.

These are proposed acceptance conditions. Root reported installed store checks in progress; this report does not certify their result. Ordinary close/reopen tests do not simulate power loss or browser eviction. Human privacy review remains necessary for persisted drafts.
