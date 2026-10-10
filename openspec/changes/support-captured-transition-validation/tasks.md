## 1. Validate

- [x] 1.1 Add captured-validator tests and record the expected function-pointer restriction before implementation.
- [x] 1.2 Add positive and negative external caller fixtures with intended diagnostic checks.

## 2. Introduce

- [x] 2.1 Store the validator in private Arc ownership and accept generic Fn callbacks at the existing setter.
- [x] 2.2 Document immutable snapshot, lock, bounded execution, non-reentrancy, and replay rules.

## 3. Verify

- [x] 3.1 Run two-Runtime isolation, rejection, panic, and replay cases against real SQLite and redb databases.
- [x] 3.2 Retain all-path transition tests and verify external function, pointer, closure, and invalid-capture callers.
- [x] 3.3 Run formatting, affected Clippy/tests, downstream checks, and strict OpenSpec validation.
- [ ] 3.4 Run the full local verifier on the combined source before integration; record exact source evidence.
