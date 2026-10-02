#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="$PWD/target"
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo run --locked --quiet > "$CARGO_TARGET_DIR/results.csv"
diff -u results.csv "$CARGO_TARGET_DIR/results.csv"
