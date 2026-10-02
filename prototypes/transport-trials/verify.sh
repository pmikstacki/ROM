#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="$PWD/target"
cargo fmt --check
cargo test --locked -- --test-threads=1
cargo clippy --locked --all-targets -- -D warnings
